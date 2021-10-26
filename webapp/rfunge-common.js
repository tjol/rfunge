import {css} from 'lit'

export const RFungeMode = {
  INACTIVE: 0,
  EDIT: 1,
  RUN: 2,
  DEBUG: 3,
  DEBUG_FINISHED: 4
}

export const COMMON_STYLES = css`

button, input[type="button"], input[type="submit"] {
  border: 1px solid var(--btn-border-color);
  border-radius: 0.5em;
  background: var(--btn-background);
  color: var(--btn-color);
  padding: 0.2em 1em;
  margin: 0.2em;
  font-size: 1em;
}

input[type="text"] {
  border: 1px solid var(--input-border-color);
  font-size: 1em;
  border-radius: 0.25em;
  padding: 0.2em;
  margin: 0.2em;
  background: inherit;
  color: inherit;
}
`
