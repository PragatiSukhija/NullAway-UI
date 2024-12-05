import React from 'react';

import Header from './Header';
import Loader from './Loader';
import Section from './Section';
import OutputPrism from './OutputPrism';

interface HighlightErrorsProps {
  children: React.ReactNode;
  label: string;
}

const HighlightErrors: React.FC<HighlightErrorsProps> = ({ label, children }) => (
  <div data-test-id="output-stderr">
    <Header label={label} />
    <OutputPrism languageCode="language-rust_errors">
      {children}
    </OutputPrism>
  </div>
);

export interface SimplePaneProps extends ReallySimplePaneProps {
  children?: React.ReactNode;
  kind: string;
}

export interface ReallySimplePaneProps {
  requestsInProgress: number;
  stdout?: string;
  stderr?: string;
  error?: string;
  progressMessage?: string;
  finishedMessage? :string;
}

const SimplePane: React.FC<SimplePaneProps> = props => (
  <div data-test-id={`output-${props.kind}`}>
    {props.requestsInProgress > 0 ? (
      <Loader progressMessage={props.progressMessage} />
    ) : (
      <div>
        <Header label="Progress" /><br/>
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
          {props.finishedMessage && !props.stderr && (
            <span style={{ fontSize: '0.9rem', fontWeight: 'normal' }}>
              {props.finishedMessage}
            </span>
          )}
        </div>
      </div>
    )}
    <Section kind="error" label="Errors">{props.error}</Section>
    <HighlightErrors label="Standard Error">{props.stderr}</HighlightErrors>
    <Section kind="stdout" label="Standard Output">{props.stdout}</Section>
    {props.children}
  </div>
);

export default SimplePane;
