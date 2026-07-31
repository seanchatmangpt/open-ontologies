//! CareFollowUp semantic typestate machine — enforcing the pastoral care workflow.
//! Derived from ontology/zoela/care-followup.ttl.

/// The consequence of a completed pastoral follow-up.
#[derive(Debug, Clone)]
pub struct CareFollowUpConsequence {
    pub outcome_notes: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// State: Follow-Up Pending (zoe:CareFollowUpPending)
#[derive(Debug, Clone, Copy)]
pub struct CareFollowUpPending;

/// State: Follow-Up In Progress (zoe:CareFollowUpActive)
#[derive(Debug, Clone, Copy)]
pub struct CareFollowUpActive;

/// State: Follow-Up Completed (zoe:CareFollowUpCompleted)
#[derive(Debug, Clone)]
pub struct CareFollowUpCompleted {
    pub consequence: CareFollowUpConsequence,
}

/// The Care Follow-Up state machine.
#[derive(Debug, Clone)]
pub struct CareFollowUp<S> {
    state: S,
}

impl CareFollowUp<CareFollowUpPending> {
    /// Initialize the care follow-up state machine.
    pub fn new() -> Self {
        Self {
            state: CareFollowUpPending,
        }
    }

    /// Transition to CareFollowUpActive (corresponds to zoe:InitiateCareFollowUp event)
    pub fn initiate(self) -> CareFollowUp<CareFollowUpActive> {
        CareFollowUp {
            state: CareFollowUpActive,
        }
    }
}

impl Default for CareFollowUp<CareFollowUpPending> {
    fn default() -> Self {
        Self::new()
    }
}

impl CareFollowUp<CareFollowUpActive> {
    /// Transition to CareFollowUpCompleted (corresponds to zoe:CompleteCareFollowUp event)
    /// This requires providing the consequence (the documented outcome of the follow-up).
    pub fn complete(self, notes: String) -> CareFollowUp<CareFollowUpCompleted> {
        let consequence = CareFollowUpConsequence {
            outcome_notes: notes,
            timestamp: chrono::Utc::now(),
        };
        CareFollowUp {
            state: CareFollowUpCompleted { consequence },
        }
    }
}

impl CareFollowUp<CareFollowUpCompleted> {
    /// Retrieve the consequence details.
    pub fn consequence(&self) -> &CareFollowUpConsequence {
        &self.state.consequence
    }
}
