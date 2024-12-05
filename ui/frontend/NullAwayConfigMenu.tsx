import React, { Fragment, useCallback, useState } from 'react';
import MenuGroup from './MenuGroup';
import { CheckboxConfig } from './ConfigElement';
import { SegmentedButton } from './SegmentedButton';
import HeaderButton from './HeaderButton';
import { BuildIcon } from './Icon';
import * as actions from './actions';
import {NullAwayConfigData} from './types';
import {useDispatch} from 'react-redux';

interface BuildMenuProps {
  close: () => void;
}


const NullAwayConfigMenu: React.FC<BuildMenuProps> = (props) => {
  const [castToNonNullMethod, setCastToNonNullMethod] = useState('');
  const [checkOptionalEmptiness, setCheckOptionalEmptiness] = useState(false);
  const [checkContracts, setCheckContracts] = useState(false);
  const [jSpecifyMode, setJSpecifyMode] = useState(false);

  const nullawayConfigData: NullAwayConfigData = {
    castToNonNullMethod,
    checkOptionalEmptiness,
    checkContracts,
    jSpecifyMode,
  };

  const dispatch = useDispatch();
  const handleBuild = useCallback(() => {
    dispatch(actions.performNullAwayCompile(nullawayConfigData)); // Dispatch the action
    props.close(); // Close the prompt
  }, [dispatch, nullawayConfigData, props]);

  return (
    <Fragment>
      <MenuGroup title="NullAway Config">
        <div className="config-item">
          <label>
              CastToNonNullMethod&nbsp;&nbsp;
            <input
              type="text"
              value={castToNonNullMethod}
              onChange={(e) => setCastToNonNullMethod(e.target.value)}
              className="config-input"
            />
          </label>
        </div>
        <CheckboxConfig
          name="&nbsp;&nbsp;CheckOptionalEmptiness"
          checked={checkOptionalEmptiness}
          onChange={() => setCheckOptionalEmptiness(!checkOptionalEmptiness)}
        />
        <CheckboxConfig
          name="&nbsp;&nbsp;CheckContracts"
          checked={checkContracts}
          onChange={() => setCheckContracts(!checkContracts)}
        />
        <CheckboxConfig
          name="&nbsp;&nbsp;JSpecifyMode"
          checked={jSpecifyMode}
          onChange={() => setJSpecifyMode(!jSpecifyMode)}
        />
      </MenuGroup>

      <MenuGroup title="Action">
        <SegmentedButton isBuild onClick={handleBuild}> {/* Pass configData here */}
          <HeaderButton bold rightIcon={<BuildIcon />}>
              Build
          </HeaderButton>
        </SegmentedButton>
      </MenuGroup>
    </Fragment>
  );
};

export default NullAwayConfigMenu;
