//! SPARQL Inference Rule Validation and Safe Reasoning
//!
//! This module provides comprehensive validation and safety mechanisms for
//! SPARQL inference rules and reasoning processes. It includes:
//! - Rule syntax validation and termination checking
//! - Safe execution with limits and rollback
//! - Dependency analysis and stratification
//! - Provenance tracking for inferred triples
//! - Materialization management strategies

use anyhow::{anyhow, Result};
use indexmap::{IndexMap, IndexSet};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

// ============================================================================
// Core Data Structures
// ============================================================================

/// Represents a SPARQL inference rule with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRule {
    pub id: String,
    pub name: String,
    pub construct_query: String,
    pub where_clause: String,
    pub priority: i32,
    pub enabled: bool,
    pub dependencies: Vec<String>,
}

/// Represents an RDF triple
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Triple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// Provenance information for an inferred triple
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub triple: Triple,
    pub rule_id: String,
    pub source_triples: Vec<Triple>,
    pub inferred_at: Instant,
    pub confidence: f64,
}

/// Validation error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Syntax error in rule {rule_id}: {message}")]
    SyntaxError { rule_id: String, message: String },
    
    #[error("Infinite loop detected in rule chain: {cycle}")]
    InfiniteLoop { cycle: String },
    
    #[error("Circular dependency detected: {cycle}")]
    CircularDependency { cycle: String },
    
    #[error("Termination not guaranteed for rule {rule_id}")]
    TerminationNotGuaranteed { rule_id: String },
    
    #[error("Contradiction detected: {message}")]
    Contradiction { message: String },
    
    #[error("Memory limit exceeded: {current} > {limit}")]
    MemoryLimitExceeded { current: usize, limit: usize },
    
    #[error("Timeout exceeded: {duration:?}")]
    TimeoutExceeded { duration: Duration },
    
    #[error("Iteration limit exceeded: {iterations}")]
    IterationLimitExceeded { iterations: usize },
}

// ============================================================================
// 1. InferenceRuleValidator
// ============================================================================

/// Validates inference rules for syntax, termination, and consistency
pub struct InferenceRuleValidator {
    max_recursion_depth: usize,
    safe_predicates: HashSet<String>,
}

impl InferenceRuleValidator {
    pub fn new() -> Self {
        Self {
            max_recursion_depth: 10,
            safe_predicates: Self::default_safe_predicates(),
        }
    }

    fn default_safe_predicates() -> HashSet<String> {
        let mut set = HashSet::new();
        // RDFS predicates
        set.insert("rdfs:subClassOf".to_string());
        set.insert("rdfs:subPropertyOf".to_string());
        set.insert("rdfs:domain".to_string());
        set.insert("rdfs:range".to_string());
        set.insert("rdf:type".to_string());
        // OWL predicates
        set.insert("owl:sameAs".to_string());
        set.insert("owl:equivalentClass".to_string());
        set.insert("owl:equivalentProperty".to_string());
        set
    }

    /// Validate a single inference rule
    pub fn validate_rule(&self, rule: &InferenceRule) -> Result<(), ValidationError> {
        // Syntax validation
        self.validate_syntax(rule)?;
        
        // Check for monotonicity (safe for forward chaining)
        self.check_monotonicity(rule)?;
        
        // Validate variable safety (all variables in CONSTRUCT appear in WHERE)
        self.validate_variable_safety(rule)?;
        
        Ok(())
    }

    /// Validate SPARQL syntax
    fn validate_syntax(&self, rule: &InferenceRule) -> Result<(), ValidationError> {
        // Check CONSTRUCT clause
        if rule.construct_query.is_empty() {
            return Err(ValidationError::SyntaxError {
                rule_id: rule.id.clone(),
                message: "CONSTRUCT clause is empty".to_string(),
            });
        }

        // Check for balanced braces
        let open_braces = rule.construct_query.matches('{').count();
        let close_braces = rule.construct_query.matches('}').count();
        if open_braces != close_braces {
            return Err(ValidationError::SyntaxError {
                rule_id: rule.id.clone(),
                message: format!("Unbalanced braces: {} open, {} close", open_braces, close_braces),
            });
        }

        // Check WHERE clause
        if rule.where_clause.is_empty() {
            return Err(ValidationError::SyntaxError {
                rule_id: rule.id.clone(),
                message: "WHERE clause is empty".to_string(),
            });
        }

        Ok(())
    }

    /// Check monotonicity - rule should only add facts, not remove
    fn check_monotonicity(&self, rule: &InferenceRule) -> Result<(), ValidationError> {
        // Check for unsafe patterns (MINUS, NOT EXISTS in CONSTRUCT)
        let query_lower = rule.construct_query.to_lowercase();
        
        if query_lower.contains("minus") || query_lower.contains("not exists") {
            return Err(ValidationError::SyntaxError {
                rule_id: rule.id.clone(),
                message: "Non-monotonic operators (MINUS, NOT EXISTS) not allowed in CONSTRUCT".to_string(),
            });
        }

        Ok(())
    }

    /// Validate variable safety (all CONSTRUCT vars must appear in WHERE)
    fn validate_variable_safety(&self, rule: &InferenceRule) -> Result<(), ValidationError> {
        let construct_vars = self.extract_variables(&rule.construct_query);
        let where_vars = self.extract_variables(&rule.where_clause);

        let unsafe_vars: Vec<_> = construct_vars
            .iter()
            .filter(|v| !where_vars.contains(*v))
            .collect();

        if !unsafe_vars.is_empty() {
            return Err(ValidationError::SyntaxError {
                rule_id: rule.id.clone(),
                message: format!("Variables {} in CONSTRUCT not bound in WHERE", 
                    unsafe_vars.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")),
            });
        }

        Ok(())
    }

    /// Extract variables from SPARQL query
    fn extract_variables(&self, query: &str) -> HashSet<String> {
        let var_regex = Regex::new(r"\?(\w+)").unwrap();
        var_regex
            .captures_iter(query)
            .map(|cap| cap[1].to_string())
            .collect()
    }

    /// Detect infinite loops in inference chains
    pub fn detect_infinite_loops(&self, rules: &[InferenceRule]) -> Result<(), ValidationError> {
        let dep_analyzer = RuleDependencyAnalyzer::new();
        let graph = dep_analyzer.build_dependency_graph(rules);
        
        if let Some(cycle) = dep_analyzer.find_cycle(&graph) {
            return Err(ValidationError::InfiniteLoop {
                cycle: cycle.join(" -> "),
            });
        }

        Ok(())
    }

    /// Check termination guarantees
    pub fn check_termination(&self, rules: &[InferenceRule]) -> Result<(), ValidationError> {
        // For Datalog-like rules, termination is guaranteed if:
        // 1. Rules are monotonic (checked above)
        // 2. No unbounded recursion
        // 3. Finite domain

        for rule in rules {
            // Check for recursive rules that reference themselves
            if rule.dependencies.contains(&rule.id) {
                // Recursive rules must have a bounded pattern
                if !self.has_bounded_recursion(rule) {
                    return Err(ValidationError::TerminationNotGuaranteed {
                        rule_id: rule.id.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Check if recursive rule has bounded recursion
    fn has_bounded_recursion(&self, rule: &InferenceRule) -> bool {
        // Heuristic: check for base case patterns
        let where_lower = rule.where_clause.to_lowercase();
        
        // Safe patterns that guarantee termination
        where_lower.contains("filter") || 
        where_lower.contains("bound(") ||
        where_lower.contains("exists") ||
        self.safe_predicates.iter().any(|p| where_lower.contains(p))
    }

    /// Validate rule priorities
    pub fn validate_priorities(&self, rules: &[InferenceRule]) -> Result<(), ValidationError> {
        // Check for conflicting priorities
        let mut priority_map: HashMap<i32, Vec<String>> = HashMap::new();
        
        for rule in rules {
            priority_map
                .entry(rule.priority)
                .or_insert_with(Vec::new)
                .push(rule.id.clone());
        }

        // Rules with same priority should not have dependencies on each other
        for (priority, rule_ids) in &priority_map {
            if rule_ids.len() > 1 {
                // Check if any have mutual dependencies
                for i in 0..rule_ids.len() {
                    for j in (i + 1)..rule_ids.len() {
                        let rule_i = rules.iter().find(|r| r.id == rule_ids[i]).unwrap();
                        let rule_j = rules.iter().find(|r| r.id == rule_ids[j]).unwrap();
                        
                        if rule_i.dependencies.contains(&rule_ids[j]) || 
                           rule_j.dependencies.contains(&rule_ids[i]) {
                            tracing::warn!(
                                "Rules {} and {} have same priority {} but are dependent",
                                rule_ids[i], rule_ids[j], priority
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for InferenceRuleValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 2. ReasoningGuard
// ============================================================================

/// Configuration for safe reasoning execution
#[derive(Debug, Clone)]
pub struct ReasoningConfig {
    pub max_iterations: usize,
    pub timeout: Duration,
    pub max_inferred_triples: usize,
    pub checkpoint_interval: usize,
    pub enable_rollback: bool,
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            max_iterations: 1000,
            timeout: Duration::from_secs(60),
            max_inferred_triples: 100_000,
            checkpoint_interval: 100,
            enable_rollback: true,
        }
    }
}

/// Guards the reasoning process with safety limits
pub struct ReasoningGuard {
    config: ReasoningConfig,
    start_time: Instant,
    iterations: usize,
    inferred_count: usize,
    checkpoints: Vec<ReasoningCheckpoint>,
}

#[derive(Debug, Clone)]
struct ReasoningCheckpoint {
    iteration: usize,
    triple_count: usize,
    timestamp: Instant,
}

impl ReasoningGuard {
    pub fn new(config: ReasoningConfig) -> Self {
        Self {
            config,
            start_time: Instant::now(),
            iterations: 0,
            inferred_count: 0,
            checkpoints: Vec::new(),
        }
    }

    /// Check if we should continue reasoning
    pub fn check_continue(&mut self) -> Result<(), ValidationError> {
        // Check timeout
        let elapsed = self.start_time.elapsed();
        if elapsed > self.config.timeout {
            return Err(ValidationError::TimeoutExceeded { duration: elapsed });
        }

        // Check iteration limit
        if self.iterations >= self.config.max_iterations {
            return Err(ValidationError::IterationLimitExceeded {
                iterations: self.iterations,
            });
        }

        // Check memory limit
        if self.inferred_count >= self.config.max_inferred_triples {
            return Err(ValidationError::MemoryLimitExceeded {
                current: self.inferred_count,
                limit: self.config.max_inferred_triples,
            });
        }

        Ok(())
    }

    /// Record an iteration
    pub fn record_iteration(&mut self, new_triples: usize) {
        self.iterations += 1;
        self.inferred_count += new_triples;

        // Create checkpoint if needed
        if self.config.enable_rollback && 
           self.iterations % self.config.checkpoint_interval == 0 {
            self.checkpoints.push(ReasoningCheckpoint {
                iteration: self.iterations,
                triple_count: self.inferred_count,
                timestamp: Instant::now(),
            });
        }
    }

    /// Get reasoning statistics
    pub fn get_stats(&self) -> ReasoningStats {
        ReasoningStats {
            iterations: self.iterations,
            inferred_triples: self.inferred_count,
            elapsed: self.start_time.elapsed(),
            checkpoints: self.checkpoints.len(),
        }
    }

    /// Rollback to last checkpoint
    pub fn get_last_checkpoint(&self) -> Option<&ReasoningCheckpoint> {
        self.checkpoints.last()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStats {
    pub iterations: usize,
    pub inferred_triples: usize,
    pub elapsed: Duration,
    pub checkpoints: usize,
}

// ============================================================================
// 3. RuleDependencyAnalyzer
// ============================================================================

/// Analyzes dependencies between inference rules
pub struct RuleDependencyAnalyzer {
    _placeholder: (),
}

impl RuleDependencyAnalyzer {
    pub fn new() -> Self {
        Self { _placeholder: () }
    }

    /// Build dependency graph from rules
    pub fn build_dependency_graph(&self, rules: &[InferenceRule]) -> DependencyGraph {
        let mut graph = DependencyGraph::new();

        for rule in rules {
            graph.add_node(rule.id.clone());
            
            for dep_id in &rule.dependencies {
                graph.add_edge(dep_id.clone(), rule.id.clone());
            }
        }

        graph
    }

    /// Find circular dependencies
    pub fn find_cycle(&self, graph: &DependencyGraph) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node in graph.nodes.keys() {
            if let Some(cycle) = self.dfs_cycle(node, graph, &mut visited, &mut rec_stack, &mut Vec::new()) {
                return Some(cycle);
            }
        }

        None
    }

    fn dfs_cycle(
        &self,
        node: &str,
        graph: &DependencyGraph,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        if rec_stack.contains(node) {
            // Found cycle - build cycle path
            let cycle_start = path.iter().position(|n| n == node).unwrap();
            return Some(path[cycle_start..].to_vec());
        }

        if visited.contains(node) {
            return None;
        }

        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = graph.nodes.get(node) {
            for neighbor in neighbors {
                if let Some(cycle) = self.dfs_cycle(neighbor, graph, visited, rec_stack, path) {
                    return Some(cycle);
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
        None
    }

    /// Topological sort for execution order
    pub fn topological_sort(&self, graph: &DependencyGraph) -> Result<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut result = Vec::new();

        // Calculate in-degrees
        for node in graph.nodes.keys() {
            in_degree.insert(node.clone(), 0);
        }

        for edges in graph.nodes.values() {
            for target in edges {
                *in_degree.get_mut(target).unwrap() += 1;
            }
        }

        // Queue of nodes with in-degree 0
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(node, _)| node.clone())
            .collect();

        while let Some(node) = queue.pop_front() {
            result.push(node.clone());

            if let Some(neighbors) = graph.nodes.get(&node) {
                for neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        // Check if all nodes were processed
        if result.len() != graph.nodes.len() {
            return Err(anyhow!("Cycle detected in dependency graph"));
        }

        Ok(result)
    }

    /// Stratification for recursive rules
    pub fn stratify(&self, rules: &[InferenceRule]) -> Result<Vec<Vec<String>>> {
        let graph = self.build_dependency_graph(rules);
        let sorted = self.topological_sort(&graph)?;

        // Group rules into strata
        let mut strata: Vec<Vec<String>> = Vec::new();
        let mut current_stratum = Vec::new();
        let mut processed = HashSet::new();

        for rule_id in sorted {
            let rule = rules.iter().find(|r| r.id == rule_id).unwrap();
            
            // Check if any dependency is in current stratum
            let depends_on_current = rule.dependencies
                .iter()
                .any(|dep| current_stratum.contains(dep));

            if depends_on_current && !current_stratum.is_empty() {
                // Start new stratum
                strata.push(current_stratum.clone());
                current_stratum.clear();
            }

            current_stratum.push(rule_id.clone());
            processed.insert(rule_id);
        }

        if !current_stratum.is_empty() {
            strata.push(current_stratum);
        }

        Ok(strata)
    }

    /// Optimize execution order based on dependencies and priorities
    pub fn optimize_execution_order(&self, rules: &[InferenceRule]) -> Result<Vec<String>> {
        // Sort by priority first, then by dependencies
        let mut sorted_rules = rules.to_vec();
        sorted_rules.sort_by_key(|r| (-r.priority, r.id.clone()));

        let graph = self.build_dependency_graph(&sorted_rules);
        self.topological_sort(&graph)
    }
}

impl Default for RuleDependencyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency graph structure
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub nodes: IndexMap<String, Vec<String>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: IndexMap::new(),
        }
    }

    pub fn add_node(&mut self, id: String) {
        self.nodes.entry(id).or_insert_with(Vec::new);
    }

    pub fn add_edge(&mut self, from: String, to: String) {
        self.nodes.entry(from).or_insert_with(Vec::new).push(to);
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 4. InferredTripleValidator
// ============================================================================

/// Validates inferred triples and tracks provenance
pub struct InferredTripleValidator {
    provenance_map: HashMap<Triple, Provenance>,
    constraints: Vec<Box<dyn TripleConstraint>>,
}

impl InferredTripleValidator {
    pub fn new() -> Self {
        Self {
            provenance_map: HashMap::new(),
            constraints: Vec::new(),
        }
    }

    /// Add a constraint checker
    pub fn add_constraint(&mut self, constraint: Box<dyn TripleConstraint>) {
        self.constraints.push(constraint);
    }

    /// Validate an inferred triple
    pub fn validate_triple(&self, triple: &Triple) -> Result<(), ValidationError> {
        for constraint in &self.constraints {
            constraint.check(triple)?;
        }
        Ok(())
    }

    /// Record provenance for an inferred triple
    pub fn record_provenance(
        &mut self,
        triple: Triple,
        rule_id: String,
        source_triples: Vec<Triple>,
    ) {
        let provenance = Provenance {
            triple: triple.clone(),
            rule_id,
            source_triples,
            inferred_at: Instant::now(),
            confidence: 1.0,
        };
        self.provenance_map.insert(triple, provenance);
    }

    /// Get provenance for a triple
    pub fn get_provenance(&self, triple: &Triple) -> Option<&Provenance> {
        self.provenance_map.get(triple)
    }

    /// Get justification chain for a triple
    pub fn get_justification(&self, triple: &Triple) -> Vec<Provenance> {
        let mut justification = Vec::new();
        let mut to_process = VecDeque::new();
        let mut processed = HashSet::new();

        to_process.push_back(triple.clone());

        while let Some(current) = to_process.pop_front() {
            if processed.contains(&current) {
                continue;
            }
            processed.insert(current.clone());

            if let Some(prov) = self.provenance_map.get(&current) {
                justification.push(prov.clone());
                for source in &prov.source_triples {
                    to_process.push_back(source.clone());
                }
            }
        }

        justification
    }

    /// Detect contradictions in inferred triples
    pub fn detect_contradictions(&self, triples: &[Triple]) -> Result<(), ValidationError> {
        // Check for owl:sameAs contradictions
        for triple in triples {
            if triple.predicate == "owl:differentFrom" {
                // Check if there's also a sameAs
                let same_as = Triple {
                    subject: triple.subject.clone(),
                    predicate: "owl:sameAs".to_string(),
                    object: triple.object.clone(),
                };
                
                if triples.contains(&same_as) {
                    return Err(ValidationError::Contradiction {
                        message: format!(
                            "{} cannot be both sameAs and differentFrom {}",
                            triple.subject, triple.object
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    /// Handle retraction of an inferred triple
    pub fn retract_triple(&mut self, triple: &Triple) -> Vec<Triple> {
        let mut to_retract = Vec::new();
        
        // Find all triples that depend on this one
        for (t, prov) in &self.provenance_map {
            if prov.source_triples.contains(triple) {
                to_retract.push(t.clone());
            }
        }

        // Remove from provenance map
        self.provenance_map.remove(triple);

        // Recursively retract dependent triples
        for dependent in &to_retract {
            let mut transitive = self.retract_triple(dependent);
            to_retract.append(&mut transitive);
        }

        to_retract
    }
}

impl Default for InferredTripleValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for triple constraints
pub trait TripleConstraint: Send + Sync {
    fn check(&self, triple: &Triple) -> Result<(), ValidationError>;
}

/// Example constraint: prevent certain predicates
pub struct PredicateBlacklist {
    blacklist: HashSet<String>,
}

impl PredicateBlacklist {
    pub fn new(predicates: Vec<String>) -> Self {
        Self {
            blacklist: predicates.into_iter().collect(),
        }
    }
}

impl TripleConstraint for PredicateBlacklist {
    fn check(&self, triple: &Triple) -> Result<(), ValidationError> {
        if self.blacklist.contains(&triple.predicate) {
            return Err(ValidationError::Contradiction {
                message: format!("Predicate {} is blacklisted", triple.predicate),
            });
        }
        Ok(())
    }
}

// ============================================================================
// 5. MaterializationManager
// ============================================================================

/// Strategy for materialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializationStrategy {
    /// Materialize all inferences immediately
    Eager,
    /// Materialize only when queried
    Lazy,
    /// Materialize selectively based on heuristics
    Selective,
    /// Hybrid approach
    Hybrid,
}

/// Configuration for materialization
#[derive(Debug, Clone)]
pub struct MaterializationConfig {
    pub strategy: MaterializationStrategy,
    pub max_materialized: usize,
    pub invalidation_strategy: InvalidationStrategy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationStrategy {
    /// Invalidate all on any change
    Full,
    /// Invalidate only affected triples
    Incremental,
    /// Use timestamps
    Timestamp,
}

impl Default for MaterializationConfig {
    fn default() -> Self {
        Self {
            strategy: MaterializationStrategy::Selective,
            max_materialized: 50_000,
            invalidation_strategy: InvalidationStrategy::Incremental,
        }
    }
}

/// Manages materialized inferences
pub struct MaterializationManager {
    config: MaterializationConfig,
    materialized: IndexSet<Triple>,
    query_frequency: HashMap<Triple, usize>,
    last_update: Instant,
}

impl MaterializationManager {
    pub fn new(config: MaterializationConfig) -> Self {
        Self {
            config,
            materialized: IndexSet::new(),
            query_frequency: HashMap::new(),
            last_update: Instant::now(),
        }
    }

    /// Decide whether to materialize a triple
    pub fn should_materialize(&mut self, triple: &Triple) -> bool {
        match self.config.strategy {
            MaterializationStrategy::Eager => {
                self.materialized.len() < self.config.max_materialized
            }
            MaterializationStrategy::Lazy => false,
            MaterializationStrategy::Selective => {
                // Materialize if queried frequently
                let freq = self.query_frequency.get(triple).unwrap_or(&0);
                *freq > 3 && self.materialized.len() < self.config.max_materialized
            }
            MaterializationStrategy::Hybrid => {
                // Materialize common inference patterns
                self.is_common_pattern(triple) && 
                self.materialized.len() < self.config.max_materialized
            }
        }
    }

    fn is_common_pattern(&self, triple: &Triple) -> bool {
        // Common patterns to materialize
        matches!(
            triple.predicate.as_str(),
            "rdf:type" | "rdfs:subClassOf" | "rdfs:subPropertyOf" | "owl:sameAs"
        )
    }

    /// Materialize a triple
    pub fn materialize(&mut self, triple: Triple) {
        if self.materialized.len() < self.config.max_materialized {
            self.materialized.insert(triple);
        } else {
            // Evict least frequently queried
            self.evict_lfu();
            self.materialized.insert(triple);
        }
    }

    fn evict_lfu(&mut self) {
        if let Some(lfu_triple) = self.find_least_frequent() {
            self.materialized.shift_remove(&lfu_triple);
            self.query_frequency.remove(&lfu_triple);
        }
    }

    fn find_least_frequent(&self) -> Option<Triple> {
        self.materialized
            .iter()
            .min_by_key(|t| self.query_frequency.get(*t).unwrap_or(&0))
            .cloned()
    }

    /// Record a query for a triple
    pub fn record_query(&mut self, triple: &Triple) {
        *self.query_frequency.entry(triple.clone()).or_insert(0) += 1;
    }

    /// Check if a triple is materialized
    pub fn is_materialized(&self, triple: &Triple) -> bool {
        self.materialized.contains(triple)
    }

    /// Invalidate based on strategy
    pub fn invalidate(&mut self, changed_triples: &[Triple]) {
        match self.config.invalidation_strategy {
            InvalidationStrategy::Full => {
                self.materialized.clear();
                self.query_frequency.clear();
            }
            InvalidationStrategy::Incremental => {
                // Remove affected triples
                for triple in changed_triples {
                    self.materialized.shift_remove(triple);
                }
            }
            InvalidationStrategy::Timestamp => {
                // Mark as stale, actual invalidation happens on query
                self.last_update = Instant::now();
            }
        }
    }

    /// Get materialization statistics
    pub fn get_stats(&self) -> MaterializationStats {
        MaterializationStats {
            materialized_count: self.materialized.len(),
            query_count: self.query_frequency.values().sum(),
            last_update: self.last_update,
        }
    }

    /// Optimize storage by removing rarely used materializations
    pub fn optimize_storage(&mut self) {
        let threshold = 2;
        let to_remove: Vec<_> = self
            .materialized
            .iter()
            .filter(|t| self.query_frequency.get(*t).unwrap_or(&0) < &threshold)
            .cloned()
            .collect();

        for triple in to_remove {
            self.materialized.shift_remove(&triple);
            self.query_frequency.remove(&triple);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializationStats {
    pub materialized_count: usize,
    pub query_count: usize,
    pub last_update: Instant,
}

// ============================================================================
// Helper Functions and Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_validator_syntax() {
        let validator = InferenceRuleValidator::new();
        
        let valid_rule = InferenceRule {
            id: "rule1".to_string(),
            name: "Test Rule".to_string(),
            construct_query: "CONSTRUCT { ?s rdf:type ?t } ".to_string(),
            where_clause: "WHERE { ?s rdfs:subClassOf ?t }".to_string(),
            priority: 1,
            enabled: true,
            dependencies: vec![],
        };

        assert!(validator.validate_rule(&valid_rule).is_ok());
    }

    #[test]
    fn test_circular_dependency_detection() {
        let analyzer = RuleDependencyAnalyzer::new();
        
        let rules = vec![
            InferenceRule {
                id: "rule1".to_string(),
                name: "Rule 1".to_string(),
                construct_query: "CONSTRUCT { ?s ?p ?o }".to_string(),
                where_clause: "WHERE { ?s ?p ?o }".to_string(),
                priority: 1,
                enabled: true,
                dependencies: vec!["rule2".to_string()],
            },
            InferenceRule {
                id: "rule2".to_string(),
                name: "Rule 2".to_string(),
                construct_query: "CONSTRUCT { ?s ?p ?o }".to_string(),
                where_clause: "WHERE { ?s ?p ?o }".to_string(),
                priority: 1,
                enabled: true,
                dependencies: vec!["rule1".to_string()],
            },
        ];

        let graph = analyzer.build_dependency_graph(&rules);
        assert!(analyzer.find_cycle(&graph).is_some());
    }

    #[test]
    fn test_reasoning_guard() {
        let config = ReasoningConfig {
            max_iterations: 10,
            timeout: Duration::from_secs(5),
            max_inferred_triples: 100,
            checkpoint_interval: 5,
            enable_rollback: true,
        };

        let mut guard = ReasoningGuard::new(config);

        for i in 0..5 {
            assert!(guard.check_continue().is_ok());
            guard.record_iteration(10);
        }

        let stats = guard.get_stats();
        assert_eq!(stats.iterations, 5);
        assert_eq!(stats.inferred_triples, 50);
    }

    #[test]
    fn test_provenance_tracking() {
        let mut validator = InferredTripleValidator::new();

        let source = Triple {
            subject: "ex:A".to_string(),
            predicate: "rdfs:subClassOf".to_string(),
            object: "ex:B".to_string(),
        };

        let inferred = Triple {
            subject: "ex:instance".to_string(),
            predicate: "rdf:type".to_string(),
            object: "ex:B".to_string(),
        };

        validator.record_provenance(
            inferred.clone(),
            "rule1".to_string(),
            vec![source],
        );

        assert!(validator.get_provenance(&inferred).is_some());
    }

    #[test]
    fn test_materialization_strategy() {
        let config = MaterializationConfig {
            strategy: MaterializationStrategy::Selective,
            max_materialized: 10,
            invalidation_strategy: InvalidationStrategy::Incremental,
        };

        let mut manager = MaterializationManager::new(config);

        let triple = Triple {
            subject: "ex:A".to_string(),
            predicate: "rdf:type".to_string(),
            object: "ex:Class".to_string(),
        };

        // Record multiple queries to make it frequent
        for _ in 0..5 {
            manager.record_query(&triple);
        }

        assert!(manager.should_materialize(&triple));
    }
}
