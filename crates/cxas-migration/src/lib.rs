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

mod dfcx;
mod error;
mod hillclimb;
mod pipeline;
mod snapshot;
mod tool_sync;

pub use dfcx::{
    AIAugment, ConversationTrace, ConversationTurn, ConversationalAgentsAPI, DFCXAgentExporter,
    DFCXConversationRunner, FlowDependencyResolver, FlowTreeVisualizer, HighLevelGraphVisualizer,
    IrBundle, MainVisualizer, PlaybookTreeVisualizer,
};
pub use error::{LifeError, MigrateError};
pub use hillclimb::HillclimbRun;
pub use pipeline::{MigratedApp, MigrationPipeline, MigrationSource, MigrationTarget, Profile};
pub use snapshot::{take_failed_deletes, CleanupQueue, SnapshotApi, SnapshotGuard, SnapshotName};
pub use tool_sync::{SyncReport, ToolSync};

#[cfg(feature = "tui")]
pub use pipeline::MigrationTui;

pub fn crate_name() -> &'static str {
    "cxas-migration"
}
