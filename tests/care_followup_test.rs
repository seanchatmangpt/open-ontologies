#[cfg(test)]
mod tests {
    use open_ontologies::care_followup::CareFollowUp;

    #[test]
    fn test_care_followup_transition_flow() {
        // Initialize in Pending state
        let task = CareFollowUp::new();

        // Transition using Initiate event to Active state
        let task_active = task.initiate();

        // Transition using Complete event to Completed state with consequence notes
        let task_completed = task_active.complete("Pastoral check completed. Family is doing well and needs no physical assistance at this time.".to_string());

        // Assert the consequence was generated correctly
        let consequence = task_completed.consequence();
        assert_eq!(
            consequence.outcome_notes,
            "Pastoral check completed. Family is doing well and needs no physical assistance at this time."
        );
        assert!(consequence.timestamp <= chrono::Utc::now());
    }
}
