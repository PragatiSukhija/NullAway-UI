import React, { Fragment, useCallback, useState } from 'react';
import MenuGroup from './MenuGroup';
import { SegmentedButton } from './SegmentedButton';
import HeaderButton from './HeaderButton';
import { BuildIcon } from './Icon';
import * as actions from './actions';
import {AnnotatorConfigData} from './types';
import {useDispatch} from 'react-redux';
import {CheckboxConfig} from './ConfigElement';


interface BuildMenuProps {
  close: () => void;
}


const AnnotatorMenu: React.FC<BuildMenuProps> = (props) => {
  const [nullUnmarked, setNullUnmarked] = useState(false);


  const annotatorConfigData: AnnotatorConfigData = {
    nullUnmarked,
  };

  const dispatch = useDispatch();
  const handleBuild = useCallback(() => {
    dispatch(actions.runAnnotator(annotatorConfigData));
    props.close(); // Close the prompt
  }, [dispatch, annotatorConfigData, props]);

  return (
    <Fragment>
      <MenuGroup title="Annotator Config">
        <div className="config-item">
          <CheckboxConfig
            name="&nbsp;&nbsp;Suppress remaining errors"
            checked={nullUnmarked}
            onChange={() => setNullUnmarked(!nullUnmarked)}
          />
        </div>
      </MenuGroup>

      <MenuGroup title="Action">
        <SegmentedButton isBuild onClick={handleBuild}> {/* Pass configData here */}
          <HeaderButton bold rightIcon={<BuildIcon />}>
                        Annotate
          </HeaderButton>
        </SegmentedButton>
      </MenuGroup>
    </Fragment>
  );
};

export default AnnotatorMenu;
