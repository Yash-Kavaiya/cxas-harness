// Copyright 2026 The cxas-harness Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use cxas_evals::{TurnCursor, UserInput};

#[test]
fn next_advances_instead_of_repeating_the_first_utterance() {
    let mut c = TurnCursor::new(vec![
        UserInput::Text("alpha".into()),
        UserInput::Text("beta".into()),
        UserInput::Text("gamma".into()),
    ]);
    assert_eq!(c.next(), Some(&UserInput::Text("alpha".into())));
    assert_eq!(c.next(), Some(&UserInput::Text("beta".into())));
    assert_eq!(c.next(), Some(&UserInput::Text("gamma".into())));
    assert_eq!(c.next(), None);
    assert!(c.is_exhausted());
}

#[test]
fn peek_does_not_advance() {
    let c = TurnCursor::new(vec![UserInput::Text("only".into())]);
    assert_eq!(c.peek(), Some(&UserInput::Text("only".into())));
    assert_eq!(c.remaining(), 1);
}
