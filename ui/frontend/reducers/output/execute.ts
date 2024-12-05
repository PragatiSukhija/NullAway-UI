import { createAsyncThunk, createSlice } from '@reduxjs/toolkit';
import * as z from 'zod';

import { SimpleThunkAction, adaptFetchError, jsonPost, routes } from '../../actions';
import { executeRequestPayloadSelector, useWebsocketSelector } from '../../selectors';
import {AnnotatorConfigData, NullAwayConfigData, Release, Runtime} from '../../types';
import {
  WsPayloadAction,
  createWebsocketResponseAction,
  createWebsocketResponseSchema,
  makeWebSocketMeta,
} from '../../websocketActions';

const initialState: State = {
  requestsInProgress: 0,
};

interface State {
  sequenceNumber?: number;
  requestsInProgress: number;
  stdout?: string;
  stderr?: string;
  error?: string;
}

const wsExecuteResponsePayloadSchema = z.object({
  success: z.boolean(),
  stdout: z.string(),
  stderr: z.string(),
});
type wsExecuteResponsePayload = z.infer<typeof wsExecuteResponsePayloadSchema>;

type wsExecuteRequestPayload = {
  runtime: Runtime;
  release: Release;
  action: string;
  code: string;
};

const wsExecuteResponse = createWebsocketResponseAction<wsExecuteResponsePayload>(
  'output/execute/wsExecuteResponse',
);

const sliceName = 'output/execute';

export interface ExecuteRequestBody {
  runtime: string;
  action: string;
  code: string;
  release: string;
  preview: boolean;
}

interface ExecuteResponseBody {
  success: boolean;
  stdout: string;
  stderr: string;
}

export const performExecute = createAsyncThunk(sliceName, async (payload: ExecuteRequestBody) =>
  adaptFetchError(() => jsonPost<ExecuteResponseBody>(routes.execute, payload)),
);

const slice = createSlice({
  name: 'output/execute',
  initialState,
  reducers: {
    wsExecuteRequest: {
      reducer: (state, action: WsPayloadAction<wsExecuteRequestPayload>) => {
        const { sequenceNumber } = action.meta;
        if (sequenceNumber >= (state.sequenceNumber ?? 0)) {
          state.sequenceNumber = sequenceNumber;
          state.requestsInProgress = 1; // Only tracking one request
        }
      },

      prepare: (payload: wsExecuteRequestPayload) => ({
        payload,
        meta: makeWebSocketMeta(),
      }),
    },

    resetStdout: (state) => {
      state.stdout = undefined;
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(performExecute.pending, (state) => {
        state.requestsInProgress += 1;
      })
      .addCase(performExecute.fulfilled, (state, action) => {
        const { stdout } = action.payload;
        let { stderr } = action.payload;
        stderr = stderr?.replace(
          'Note: Main.java uses preview features of Java SE 22.\n' +
            'Note: Recompile with -Xlint:preview for details.\n\n',
          '',
        );
        Object.assign(state, { stdout, stderr });
        state.requestsInProgress -= 1;
      })
      .addCase(performExecute.rejected, (state, action) => {
        if (action.payload) {
        } else {
          state.error = action.error.message;
        }
        state.requestsInProgress -= 1;
      })
      .addCase(wsExecuteResponse, (state, action) => {
        const {
          payload: { stdout, stderr },
          meta: { sequenceNumber },
        } = action;

        if (sequenceNumber >= (state.sequenceNumber ?? 0)) {
          Object.assign(state, { stdout, stderr });
          state.requestsInProgress = 0; // Only tracking one request
        }
      });
  },
});

export const { wsExecuteRequest, resetStdout } = slice.actions;

/*
export const performCommonExecute =
  (action: string): SimpleThunkAction =>
  (dispatch, getState) => {
    const state = getState();
    const body = executeRequestPayloadSelector(state, { action });
    const useWebSocket = useWebsocketSelector(state);

    if (useWebSocket) {
      dispatch(wsExecuteRequest(body));
    } else {
      dispatch(performExecute(body));
    }
  };
*/

export const performCommonExecute =
    (action: string, configData?: NullAwayConfigData, annotatorConfig?: AnnotatorConfigData): SimpleThunkAction =>
        (dispatch, getState) => {
          console.log('Annotator Config Data:', annotatorConfig);
          const state = getState();
          const body = executeRequestPayloadSelector(state, { action, configData, annotatorConfig });
          const useWebSocket = useWebsocketSelector(state);

          if (useWebSocket) {
            dispatch(wsExecuteRequest(body));
          } else {
            dispatch(performExecute(body));
          }
        };


export const wsExecuteResponseSchema = createWebsocketResponseSchema(
  wsExecuteResponse,
  wsExecuteResponsePayloadSchema,
);

export default slice.reducer;
