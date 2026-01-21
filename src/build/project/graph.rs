use crate::DatabaseEngine;
use crate::build::change::Change;
use crate::build::plan::Plan;
use daggy::petgraph::Direction;
use daggy::petgraph::visit::IntoEdgesDirected;
use daggy::{Dag, EdgeIndex, Walker};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use wherror::Error;

#[derive(Debug, Error)]
pub enum ChangeGraphError {
    #[error(
        "change \"{child}\" requires change \"{parent}\" which is disabled for engine \"{engine}\""
    )]
    ParentDisabledForEngine {
        child: String,
        parent: String,
        engine: DatabaseEngine,
    },
    #[error("rework \"{child}\" violates linearity of \"{parent}\"")]
    LinearReworkViolation { child: String, parent: String },
    #[error("node \"{child}\" requires node \"{parent}\" which does not exist")]
    MissingParent { child: String, parent: String },
    #[error("node \"{child}\" requires node \"{parent}\" which forms a cycle")]
    CycleFound { child: String, parent: String },
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum GraphNode {
    Root,
    Change(Change),
    Plan(Plan),
}

impl GraphNode {
    pub fn name(&self) -> &String {
        use std::sync::LazyLock;
        static ROOT_NAME: LazyLock<String> = LazyLock::new(|| "root".to_string());
        match self {
            GraphNode::Root => &ROOT_NAME,
            GraphNode::Change(c) => c.name(),
            GraphNode::Plan(p) => p.name(),
        }
    }
}

#[derive(Eq, PartialEq)]
enum ChangeRelationship {
    Root,
    Parent,
    Rework,
    PlanTarget,
    PlanParent,
}

impl ChangeGraphError {
    fn new_parent_disabled_for_engine_error(
        child_name: &str,
        parent_name: &str,
        engine: &DatabaseEngine,
    ) -> Self {
        Self::ParentDisabledForEngine {
            child: child_name.to_string(),
            parent: parent_name.to_string(),
            engine: engine.clone(),
        }
    }
    fn new_missing_parent_error(child: &str, parent: &str) -> Self {
        Self::MissingParent {
            child: child.to_string(),
            parent: parent.to_string(),
        }
    }

    fn new_cycle_error(child: &str, parent: &str) -> Self {
        Self::CycleFound {
            child: child.to_string(),
            parent: parent.to_string(),
        }
    }
}

pub(super) fn calculate_change_graph(
    changes: &HashMap<String, Arc<crate::build::change::ChangeInner>>,
    plans: &HashMap<String, Arc<crate::build::plan::PlanInner>>,
    state: &Arc<crate::build::project::ProjectState>,
    engine: &DatabaseEngine,
) -> Result<Vec<GraphNode>, ChangeGraphError> {
    let mut dag = Dag::<GraphNode, ChangeRelationship>::new();
    let mut change_node_idx = HashMap::new();
    let mut plan_node_idx = HashMap::new();

    let root_idx = dag.add_node(GraphNode::Root);

    for (name, inner) in changes {
        let change = Change::new(Arc::clone(inner), Arc::clone(state));
        change_node_idx.insert(name.clone(), dag.add_node(GraphNode::Change(change)));
    }

    for (name, inner) in plans {
        let plan = Plan::new(Arc::clone(inner), Arc::clone(state));
        plan_node_idx.insert(name.clone(), dag.add_node(GraphNode::Plan(plan)));
    }

    // Add edges for changes
    for change_inner in changes.values() {
        let change = Change::new(Arc::clone(change_inner), Arc::clone(state));
        let idx = change_node_idx[change.name()];

        // Handle reworks
        if let Some(predecessor) = change.reworks() {
            let parent_inner = match changes.get(predecessor) {
                Some(parent) => parent,
                None => {
                    return Err(ChangeGraphError::new_missing_parent_error(
                        change.name(),
                        predecessor,
                    ));
                }
            };
            let predecessor_idx = change_node_idx[&parent_inner.name];
            let already_reworked = dag
                .edges_directed(predecessor_idx, Direction::Outgoing)
                .any(|edge| *edge.weight() == ChangeRelationship::Rework);
            if already_reworked {
                return Err(ChangeGraphError::LinearReworkViolation {
                    child: change.name().clone(),
                    parent: predecessor.to_string(),
                });
            }
            dag.add_edge(predecessor_idx, idx, ChangeRelationship::Rework)
                .map_err(|_| {
                    ChangeGraphError::new_cycle_error(change.name(), &parent_inner.name)
                })?;
        } else if change.requires_for_engine(engine).is_empty() {
            // If it has no requirements for this engine, connect to root
            dag.add_edge(root_idx, idx, ChangeRelationship::Root)
                .expect("cycle error adding to root, this should never happen");
        }

        // Handle requirements
        for parent_name in change.requires_for_engine(engine) {
            if parent_name.starts_with('@') {
                // Dependency on a Plan
                let plan_name = &parent_name[1..];
                let parent_plan_inner = plans.get(plan_name).ok_or_else(|| {
                    ChangeGraphError::new_missing_parent_error(change.name(), &parent_name)
                })?;
                let parent_idx = plan_node_idx[&parent_plan_inner.name];
                dag.add_edge(parent_idx, idx, ChangeRelationship::Parent)
                    .map_err(|_| {
                        ChangeGraphError::new_cycle_error(change.name(), &parent_plan_inner.name)
                    })?;
            } else {
                // Dependency on a Change
                let parent_change_inner = match changes.get(&parent_name) {
                    Some(parent_inner) => {
                        let parent_change =
                            Change::new(Arc::clone(parent_inner), Arc::clone(state));
                        match parent_change.enabled_for_engine(engine) {
                            true => Ok(parent_inner),
                            false => Err(ChangeGraphError::new_parent_disabled_for_engine_error(
                                change.name(),
                                &parent_name,
                                engine,
                            )),
                        }
                    }
                    None => Err(ChangeGraphError::new_missing_parent_error(
                        change.name(),
                        &parent_name,
                    )),
                }?;
                let parent_idx = change_node_idx[&parent_change_inner.name];
                dag.add_edge(parent_idx, idx, ChangeRelationship::Parent)
                    .map_err(|_| {
                        ChangeGraphError::new_cycle_error(change.name(), &parent_change_inner.name)
                    })?;
            }
        }
    }

    // Add edges for plans
    for plan_inner in plans.values() {
        let plan = Plan::new(Arc::clone(plan_inner), Arc::clone(state));
        let idx = plan_node_idx[plan.name()];

        // Change -> Plan (targets)
        for target_name in plan.targets() {
            let target_change_inner = changes.get(target_name).ok_or_else(|| {
                ChangeGraphError::new_missing_parent_error(plan.name(), target_name)
            })?;
            let target_idx = change_node_idx[&target_change_inner.name];
            dag.add_edge(target_idx, idx, ChangeRelationship::PlanTarget)
                .map_err(|_| {
                    ChangeGraphError::new_cycle_error(plan.name(), &target_change_inner.name)
                })?;
        }

        // Plan -> Plan (parent)
        if let Some(parent_plan_name) = plan.parent() {
            let parent_plan_inner = plans.get(parent_plan_name).ok_or_else(|| {
                ChangeGraphError::new_missing_parent_error(plan.name(), parent_plan_name)
            })?;
            let parent_idx = plan_node_idx[&parent_plan_inner.name];
            dag.add_edge(parent_idx, idx, ChangeRelationship::PlanParent)
                .map_err(|_| {
                    ChangeGraphError::new_cycle_error(plan.name(), &parent_plan_inner.name)
                })?;
        }
    }

    // Connect nodes with no incoming edges to root
    // (Except root itself)
    let orphan_nodes: Vec<_> = dag
        .raw_nodes()
        .iter()
        .enumerate()
        .map(|(i, _)| daggy::NodeIndex::new(i))
        .filter(|&idx| {
            idx != root_idx
                && dag
                    .edges_directed(idx, Direction::Incoming)
                    .next()
                    .is_none()
        })
        .collect();

    for idx in orphan_nodes {
        dag.add_edge(root_idx, idx, ChangeRelationship::Root)
            .expect("cycle error adding to root, this should never happen");
    }

    let end_idx = EdgeIndex::<u32>::end();
    let leaves = dag
        .raw_nodes()
        .iter()
        .enumerate()
        .filter(|(_, node)| node.next_edge(Direction::Outgoing) == end_idx)
        .map(|(i, _)| daggy::NodeIndex::new(i))
        .collect::<Vec<_>>();

    dag.transitive_reduce(leaves);

    // Kahn's Algorithm for topological sort
    let mut in_degree = HashMap::new();
    for node_idx in dag
        .raw_nodes()
        .iter()
        .enumerate()
        .map(|(i, _)| daggy::NodeIndex::new(i))
    {
        let count = dag.parents(node_idx).iter(&dag).count();
        in_degree.insert(node_idx, count);
    }

    let mut pending = VecDeque::new();
    if in_degree.get(&root_idx) == Some(&0) {
        pending.push_back(root_idx);
    }

    let mut ordered_nodes = Vec::new();

    while let Some(node_idx) = pending.pop_front() {
        ordered_nodes.push(node_idx);

        let mut children = dag
            .children(node_idx)
            .iter(&dag)
            .map(|(_, idx)| idx)
            .collect::<Vec<_>>();

        children.sort_by_key(|&idx| dag.node_weight(idx).unwrap().name());

        for child_idx in children {
            if let Some(degree) = in_degree.get_mut(&child_idx) {
                *degree -= 1;
                if *degree == 0 {
                    pending.push_back(child_idx);
                }
            }
        }
    }

    let mut result: Vec<GraphNode> = Vec::new();

    for node_idx in ordered_nodes {
        let node = dag.node_weight(node_idx).unwrap();
        if let GraphNode::Root = node {
            continue;
        }
        result.push(node.clone());
    }

    Ok(result)
}
