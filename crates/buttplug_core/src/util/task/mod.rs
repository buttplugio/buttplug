// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Task Scope and Task Registry: ownership and introspection for spawned tasks.
//!
//! Every task is spawned through a [TaskScope], which links it into an
//! ownership tree, derives its hierarchical path, registers it in the global
//! [TaskRegistry], and hands it a cooperative [CancellationToken]. Dropping a
//! scope cancels its subtree.

mod registry;
// `scope` lands in Task 2 of the task-scope-lifecycle plan.
// mod scope;

pub use registry::{TaskEvent, TaskId, TaskInfo, TaskOutcome, TaskRegistry, registry};
// pub use scope::TaskScope;
