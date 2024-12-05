import React from 'react';

import GenericLoader from '../Loader';
import Header from './Header';


interface LoaderProps {
  progressMessage?: string;
}
const Loader: React.FC<LoaderProps> = ({ progressMessage }) => (
  <div>
    <Header label="Progress" />
    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
      {progressMessage && (
        <span style={{ fontSize: '0.9rem', fontWeight: 'normal' }}>
          {progressMessage}
        </span>
      )}
      <GenericLoader />
    </div>
  </div>
);

export default Loader;
