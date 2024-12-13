import React, { Fragment, useCallback } from 'react';
import { useSelector, useDispatch } from 'react-redux';
import MenuGroup from './MenuGroup';
import { CheckboxConfig } from './ConfigElement';
import { SegmentedButton } from './SegmentedButton';
import HeaderButton from './HeaderButton';
import { BuildIcon } from './Icon';
import * as actions from './actions';
import { NullAwayConfigData } from './types';
import State from './state';
import {setNullAwayConfig} from "./actions";
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

const NullAwayConfigMenu: React.FC<BuildMenuProps> = props => {
  const dispatch = useDispatch();
  const nullAwayConfig = useSelector((state: State) => state.configuration.nullawayConfig);
  const handleInputChange = (key: keyof NullAwayConfigData, value: any) => {
     const updatedConfig: NullAwayConfigData = {
        ...nullAwayConfig,
        [key]: value,
     };
     dispatch(setNullAwayConfig(updatedConfig));
  };


  return (
      <Fragment>
        <MenuGroup title="NullAway Config">
          <div className="config-item">
            <label>
              CastToNonNullMethod&nbsp;&nbsp;
              <input
                  type="text"
                  value={nullAwayConfig.castToNonNullMethod}
                  onChange={(e) =>
                      handleInputChange('castToNonNullMethod', e.target.value)
                  }
                  className="config-input"
              />
            </label>
          </div>
          <CheckboxConfig
              name="&nbsp;&nbsp;CheckOptionalEmptiness"
              checked={nullAwayConfig.checkOptionalEmptiness}
              onChange={() =>
                  handleInputChange(
                      'checkOptionalEmptiness',
                      !nullAwayConfig.checkOptionalEmptiness
                  )
              }
          />
          <CheckboxConfig
              name="&nbsp;&nbsp;CheckContracts"
              checked={nullAwayConfig.checkContracts}
              onChange={() =>
                  handleInputChange('checkContracts', !nullAwayConfig.checkContracts)
              }
          />
          <CheckboxConfig
              name="&nbsp;&nbsp;JSpecifyMode"
              checked={nullAwayConfig.jSpecifyMode}
              onChange={() =>
                  handleInputChange('jSpecifyMode', !nullAwayConfig.jSpecifyMode)
              }
          />
        </MenuGroup>

        <MenuGroup title="Action">
          <SegmentedButton isBuild onClick={useDispatchAndClose(actions.performNullAwayCompile, props.close)}>
            <HeaderButton bold rightIcon={<BuildIcon />}>
              Build
            </HeaderButton>
          </SegmentedButton>
        </MenuGroup>
      </Fragment>
  );
};

export default NullAwayConfigMenu;
