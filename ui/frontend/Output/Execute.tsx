import React, {useCallback, useEffect, useMemo, useState} from 'react';
import {useDispatch, useSelector} from 'react-redux';

import * as actions from '../actions';
import {editCode} from '../actions';
import * as selectors from '../selectors';
import {State} from '../reducers';

import Section from './Section';
import SimplePane from './SimplePane';

import styles from './Execute.module.css';
import {resetStdout} from '../reducers/output/execute';

const Execute: React.FC = () => {
  const details = useSelector((state: State) => state.output.execute);
  const primaryAction = useSelector((state: State) => state.configuration.primaryAction);
  const stdout = useSelector((state: State) => state.output.execute.stdout);
  const code = useSelector((state: State) => state.code);
  const dispatch = useDispatch();
  const [annotationComplete, setAnnotationComplete] = useState(false);


  useEffect(() => {
    if (primaryAction !== 'annotator') return;

    if (stdout && !annotationComplete) {
      dispatch(editCode(stdout || code));
      dispatch(resetStdout());
      setAnnotationComplete(true);
    } else if (!stdout) {
      setAnnotationComplete(false);
    }
  }, [primaryAction, stdout, annotationComplete, code, dispatch]);

  const isAutoBuild = useSelector(selectors.isAutoBuildSelector);

  const addMainFunction = useCallback(() => dispatch(actions.addMainFunction()), [dispatch]);


  const progressMessage = useMemo(() => {
    if (primaryAction === 'annotator') {
      return 'Waiting for Annotations ';
    }
    return 'Building  ';
  }, [primaryAction]);

  const finishedMessage = useMemo(() => {
    if (primaryAction === 'annotator') {
      return 'Annotated Successfully! ';
    }
    return 'Build Succeeded!';
  }, [primaryAction]);

  return (
    <SimplePane
      {...details}
      kind="execute"
      progressMessage={progressMessage} finishedMessage={finishedMessage}>
      {isAutoBuild && <Warning addMainFunction={addMainFunction} />}
    </SimplePane>
  );
};

interface WarningProps {
  addMainFunction: () => any;
}

const Warning: React.FC<WarningProps> = props => (
  <Section kind="warning" label="Warnings">
    No main method was detected, so your code was compiled
    {'\n'}
    but not run. If you’d like to execute your code, please
    {'\n'}
    <button className={styles.addMain} onClick={props.addMainFunction}>
      add a main method
    </button>
    .
  </Section>
);

export default Execute;
