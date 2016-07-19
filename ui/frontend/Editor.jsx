import React, { PropTypes } from 'react';
import AceEditor from 'react-ace';
import brace from 'brace';

import 'brace/mode/rust';
import 'brace/theme/github';
import 'brace/keybinding/emacs';
// https://github.com/securingsincity/react-ace/issues/95
import 'brace/ext/language_tools';

class SimpleEditor extends React.Component {
  onChange = e => this.props.onEditCode(e.target.value);
  trackEditor = component => this._editor = component;

  render() {
    return (
      <textarea
         ref={ this.trackEditor }
         className="editor-simple"
         name="editor-simple"
         value={ this.props.code }
         onChange={ this.onChange } />
    );
  }

  componentDidUpdate(prevProps, prevState) {
    this.gotoPosition(prevProps.position, this.props.position);
  }

  gotoPosition(oldPosition, newPosition) {
    const editor = this._editor;

    if (!newPosition || !editor) { return; }
    if (newPosition === oldPosition) { return; }

    // Subtract one as this logix is zero-based and the lines are one-based
    const line = newPosition.line - 1;
    const { code } = this.props;

    const lines = code.split('\n');

    const precedingLines = lines.slice(0, line);
    const highlightedLine = lines[line];

    // Add one to account for the newline we split on and removed
    const precedingBytes = precedingLines.map(l => l.length + 1).reduce((a, b) => a + b);
    const highlightedBytes = highlightedLine.length;

    editor.setSelectionRange(precedingBytes, precedingBytes + highlightedBytes);
  }
}

class AdvancedEditor extends React.Component {
  trackEditor = component => this._editor = component;

  render() {
    const { code, onEditCode } = this.props;

    return (
      <AceEditor
         ref={ this.trackEditor }
         mode="rust"
         theme="github"
         keyboardHandler="emacs"
         value={ code }
         onChange={ onEditCode }
         name="editor"
         width="100%"
         height="100%"
         editorProps={ { $blockScrolling: true } } />
    );
  }

  componentDidUpdate(prevProps, prevState) {
    this.gotoPosition(prevProps.position, this.props.position);
  }

  gotoPosition(oldPosition, newPosition) {
    const editor = this._editor;

    if (!newPosition || !editor) { return; }
    if (newPosition === oldPosition) { return; }

    const { line, column } = newPosition;

    // Columns are zero-indexed in ACE
    editor.editor.gotoLine(line, column - 1);
    editor.editor.focus();
  }
}

export default class Editor extends React.Component {
  render() {
    const { editor, code, position, onEditCode } = this.props;
    const SelectedEditor = editor === "simple" ? SimpleEditor : AdvancedEditor;

    return (
      <div className="editor">
        <SelectedEditor code={code} position={position} onEditCode={onEditCode} />;
      </div>
    );
  }
};

Editor.propTypes = {
  editor: PropTypes.string.isRequired,
  onEditCode: PropTypes.func.isRequired,
  code: PropTypes.string.isRequired,
  position: PropTypes.shape({
    line: PropTypes.number.isRequired,
    column: PropTypes.number.isRequired,
  }).isRequired,
};
