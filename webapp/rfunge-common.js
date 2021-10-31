/*
rfunge – a Funge-98 interpreter
Copyright © 2021 Thomas Jollans

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

import { css } from 'lit'

export const RFungeMode = {
  INACTIVE: 0,
  EDIT: 1,
  RUN: 2,
  DEBUG: 3,
  DEBUG_FINISHED: 4
}

export const COMMON_STYLES = css`
  button,
  input[type='button'],
  input[type='submit'] {
    border: 1px solid var(--btn-border-color);
    border-radius: 0.5em;
    background: var(--btn-background);
    color: var(--btn-color);
    padding: 0.2em 1em;
    margin: 0.2em;
    font-size: 1em;
  }

  input[type='text'] {
    border: 1px solid var(--input-border-color);
    font-size: 1em;
    border-radius: 0.25em;
    padding: 0.2em;
    margin: 0.2em;
    background: inherit;
    color: inherit;
  }
`
