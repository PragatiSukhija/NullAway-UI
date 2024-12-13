import React, { Fragment, useCallback, useState } from 'react';
import MenuGroup from './MenuGroup';
import { SegmentedButton } from './SegmentedButton';
import HeaderButton from './HeaderButton';
import { BuildIcon } from './Icon';
import * as actions from './actions';
import {AnnotatorConfigData} from './types';
import {useDispatch, useSelector} from 'react-redux';
import {CheckboxConfig} from './ConfigElement';
import {setAnnotatorConfig} from "./actions";
import State from './state';
import {useAppDispatch} from "./configureStore";


interface BuildMenuProps {
  close: () => void;
}

const useDispatchAndClose = (action: () => actions.ThunkAction, close: () => void) => {
  const dispatch = useAppDispatch();

  return useCallback(
      () => {
        dispatch(action());
        close();
      },
      [action, close, dispatch]
  );
}


const AnnotatorMenu: React.FC<BuildMenuProps> = props => {
  const dispatch = useDispatch();
  const annotatorConfig = useSelector((state: State) => state.configuration.annotatorConfig);
  const handleInputChange = (key: keyof AnnotatorConfigData, value: boolean) => {
    const updatedConfig: AnnotatorConfigData = {
      ...annotatorConfig,
      [key]: value,
    };
    dispatch(setAnnotatorConfig(updatedConfig));
  };

  return (
    <Fragment>
      <MenuGroup title="Annotator Config">
        <div className="config-item">
          <CheckboxConfig
            name="&nbsp;&nbsp;Suppress remaining errors"
            checked={annotatorConfig.nullUnmarked}
            onChange={() =>
                handleInputChange('nullUnmarked', !annotatorConfig.nullUnmarked)
            }
          />
        </div>
      </MenuGroup>

      <MenuGroup title="Action">
        <SegmentedButton isBuild onClick={useDispatchAndClose(actions.runAnnotator, props.close)}>
          <HeaderButton bold rightIcon={<BuildIcon />}>
                        Annotate
          </HeaderButton>
        </SegmentedButton>
      </MenuGroup>
    </Fragment>
  );
};

export default AnnotatorMenu;
