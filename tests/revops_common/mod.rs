//! Shared RevOps test fixtures — Phase 3.
//!
//! Synthetic Fortune-5 object-centric event log builder. Used by every
//! `tests/revops_*.rs` integration test. Realistic but unmistakably
//! fake (Account_A_0001 / Opportunity_Q4_Expansion_0042 / Invoice_2029_
//! 11_8841 — note the future year + leading-uppercase prefix).
//!
//! The fixture exposes 8 named scenarios mapped 1:1 to the negative
//! tests in tests/revops_negative.rs:
//!
//!   HappyPath                    full reconciled chain
//!   UnsupportedForecast          forecast without contract_executed
//!   LatePartnerAttribution       partner_registered after contract
//!   DiscountWithoutApproval      discount applied but no approval
//!   UnreconciledBooking          invoice_issued without order_created
//!   RenewalRiskUndetected        renewal_due without touchpoints
//!   RawDataLeak                  fake email-shaped payload
//!   OcelTruncated                trace cut mid-flow

#![allow(dead_code)]

use open_ontologies::ocel_store::OcelStore;

pub mod groq_mock;

pub const CANARY_GROQ_KEY: &str = "groq-canary-revops-NEVERLEAKMEXYZ-9b21";

pub const FORTUNE5_WORKFLOW: &str = "Fortune5RevOpsGovernedRelease";
pub const REQUIREMENTS_WORKFLOW: &str = "RequirementsManufacturing";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    HappyPath,
    UnsupportedForecast,
    LatePartnerAttribution,
    DiscountWithoutApproval,
    UnreconciledBooking,
    RenewalRiskUndetected,
    RawDataLeak,
    OcelTruncated,
}

impl Scenario {
    pub const fn label(self) -> &'static str {
        match self {
            Scenario::HappyPath => "happy_path",
            Scenario::UnsupportedForecast => "unsupported_forecast",
            Scenario::LatePartnerAttribution => "late_partner_attribution",
            Scenario::DiscountWithoutApproval => "discount_without_approval",
            Scenario::UnreconciledBooking => "unreconciled_booking",
            Scenario::RenewalRiskUndetected => "renewal_risk_undetected",
            Scenario::RawDataLeak => "raw_data_leak",
            Scenario::OcelTruncated => "ocel_truncated",
        }
    }
}

pub fn fake_account_id(n: u32) -> String {
    format!("Account_A_{n:04}")
}
pub fn fake_opportunity_id(label: &str, n: u32) -> String {
    format!("Opportunity_{label}_{n:04}")
}
pub fn fake_contract_id(n: u32) -> String {
    format!("Contract_Renewal_{n:04}")
}
pub fn fake_invoice_id(year: u32, month: u32, n: u32) -> String {
    format!("Invoice_{year}_{month:02}_{n:04}")
}
pub fn fake_partner_id(region: &str, n: u32) -> String {
    format!("Partner_Channel_{region}_{n:02}")
}

/// Emit a single OCEL event in the test's session/scope.
///
/// Event IDs are zero-padded with a process-monotonic sequence number
/// so `build_ocel`'s `ORDER BY event_id` returns events in emit order.
/// The `time` field also advances by 1 ms per call so any time-sorted
/// view is stable.
pub fn emit(
    store: &OcelStore,
    session: &str,
    scope: &str,
    event_type: &str,
    attrs: &[(&str, &str)],
    objects: &[(&str, &str)],
) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    let base = chrono::Utc::now();
    let bumped = base + chrono::Duration::milliseconds(n as i64);
    let now = bumped.to_rfc3339();
    let event_id = format!("{session}:{n:012}:{event_type}");
    store
        .emit_event(event_id.as_str(), event_type, &now, session, attrs, objects, Some(scope))
        .unwrap();
}

/// Build the OCEL trace for a given scenario into the supplied OcelStore.
///
/// Every scenario starts with the same `account_created` and
/// `opportunity_created` events so the broken-path scenarios share a
/// realistic origin and only diverge in the failure-mode region. The
/// HappyPath produces the full reconciled chain.
pub fn build_scenario(store: &OcelStore, session: &str, scope: &str, scenario: Scenario) {
    let acct = fake_account_id(1);
    let opp = fake_opportunity_id("Q4_Expansion", 42);
    let contract = fake_contract_id(199);
    let invoice = fake_invoice_id(2029, 11, 8841);
    let partner = fake_partner_id("West", 7);

    // Common origin events — every scenario starts here.
    emit(
        store, session, scope,
        "account_created",
        &[("account_id", &acct)],
        &[(&acct, "Account")],
    );
    emit(
        store, session, scope,
        "opportunity_created",
        &[("opportunity_id", &opp), ("account_id", &acct)],
        &[(&opp, "Opportunity"), (&acct, "Account")],
    );
    emit(
        store, session, scope,
        "forecast_submitted",
        &[("opportunity_id", &opp), ("commitment", "committed")],
        &[(&opp, "Opportunity")],
    );

    match scenario {
        Scenario::HappyPath => {
            // Full reconciled chain.
            emit(store, session, scope, "partner_registered",
                &[("partner_id", &partner), ("opportunity_id", &opp)],
                &[(&partner, "PartnerRegistration"), (&opp, "Opportunity")]);
            emit(store, session, scope, "quote_created",
                &[("opportunity_id", &opp), ("amount", "1000000")],
                &[(&opp, "Opportunity")]);
            emit(store, session, scope, "discount_approved",
                &[("opportunity_id", &opp), ("discount", "0.10"), ("approver", "VP_Sales")],
                &[(&opp, "Opportunity")]);
            emit(store, session, scope, "contract_executed",
                &[("contract_id", &contract), ("opportunity_id", &opp)],
                &[(&contract, "Contract"), (&opp, "Opportunity")]);
            emit(store, session, scope, "partner_attributed",
                &[("partner_id", &partner), ("contract_id", &contract)],
                &[(&partner, "PartnerRegistration"), (&contract, "Contract")]);
            emit(store, session, scope, "order_created",
                &[("contract_id", &contract), ("order_amount", "900000")],
                &[(&contract, "Contract")]);
            emit(store, session, scope, "invoice_issued",
                &[("invoice_id", &invoice), ("contract_id", &contract), ("amount", "900000")],
                &[(&invoice, "Invoice"), (&contract, "Contract")]);
            emit(store, session, scope, "payment_received",
                &[("invoice_id", &invoice), ("amount", "900000")],
                &[(&invoice, "Invoice")]);
            emit(store, session, scope, "revenue_milestone_completed",
                &[("contract_id", &contract), ("milestone", "go_live")],
                &[(&contract, "Contract")]);
            emit(store, session, scope, "renewal_touchpoint_completed",
                &[("contract_id", &contract), ("touchpoint", "qbr")],
                &[(&contract, "Contract")]);
        }
        Scenario::UnsupportedForecast => {
            // Forecast committed, but contract_executed never happens.
            emit(store, session, scope, "quote_created",
                &[("opportunity_id", &opp), ("amount", "1000000")],
                &[(&opp, "Opportunity")]);
            // No contract_executed → forecast cannot be classified
            // as supported revenue.
        }
        Scenario::LatePartnerAttribution => {
            // contract_executed BEFORE partner_registered.
            emit(store, session, scope, "quote_created",
                &[("opportunity_id", &opp), ("amount", "1000000")],
                &[(&opp, "Opportunity")]);
            emit(store, session, scope, "contract_executed",
                &[("contract_id", &contract), ("opportunity_id", &opp)],
                &[(&contract, "Contract"), (&opp, "Opportunity")]);
            emit(store, session, scope, "partner_registered",
                &[("partner_id", &partner), ("opportunity_id", &opp)],
                &[(&partner, "PartnerRegistration"), (&opp, "Opportunity")]);
            emit(store, session, scope, "partner_attributed",
                &[("partner_id", &partner), ("contract_id", &contract)],
                &[(&partner, "PartnerRegistration"), (&contract, "Contract")]);
        }
        Scenario::DiscountWithoutApproval => {
            // Discount applied via quote, but no discount_approved.
            emit(store, session, scope, "quote_created",
                &[("opportunity_id", &opp), ("amount", "1000000"), ("discount", "0.30")],
                &[(&opp, "Opportunity")]);
            // No discount_approved.
            emit(store, session, scope, "contract_executed",
                &[("contract_id", &contract), ("opportunity_id", &opp), ("discount_applied", "0.30")],
                &[(&contract, "Contract"), (&opp, "Opportunity")]);
        }
        Scenario::UnreconciledBooking => {
            // Invoice issued but no order_created in chain.
            emit(store, session, scope, "contract_executed",
                &[("contract_id", &contract), ("opportunity_id", &opp)],
                &[(&contract, "Contract"), (&opp, "Opportunity")]);
            // Skip order_created.
            emit(store, session, scope, "invoice_issued",
                &[("invoice_id", &invoice), ("contract_id", &contract), ("amount", "900000")],
                &[(&invoice, "Invoice"), (&contract, "Contract")]);
            emit(store, session, scope, "payment_received",
                &[("invoice_id", &invoice), ("amount", "900000")],
                &[(&invoice, "Invoice")]);
        }
        Scenario::RenewalRiskUndetected => {
            // Contract executed, renewal due, but no touchpoints.
            emit(store, session, scope, "contract_executed",
                &[("contract_id", &contract), ("opportunity_id", &opp)],
                &[(&contract, "Contract"), (&opp, "Opportunity")]);
            emit(store, session, scope, "renewal_due",
                &[("contract_id", &contract), ("days_until_renewal", "30")],
                &[(&contract, "Contract")]);
            // No renewal_touchpoint_completed.
        }
        Scenario::RawDataLeak => {
            // A scenario that smuggles a raw-email-shaped attribute into
            // the projection request. The classification refused step
            // is what the test asserts.
            emit(store, session, scope, "quote_created",
                &[("opportunity_id", &opp), ("amount", "1000000"),
                  ("contact_email", "real-customer@fortune5.example.com")],
                &[(&opp, "Opportunity")]);
        }
        Scenario::OcelTruncated => {
            // Trace cut mid-flow: quote starts but no further events.
            emit(store, session, scope, "quote_created",
                &[("opportunity_id", &opp), ("amount", "1000000")],
                &[(&opp, "Opportunity")]);
            // Truncated.
        }
    }
}

/// True if the trace satisfies the "every booked amount has a complete
/// supporting chain" invariant: any `invoice_issued` is preceded by an
/// `order_created` AND a `contract_executed` for the same contract.
pub fn booking_chain_is_reconciled(events: &[(String, std::collections::HashMap<String, String>)]) -> bool {
    let mut have_contract = std::collections::HashSet::new();
    let mut have_order = std::collections::HashSet::new();
    for (etype, attrs) in events {
        match etype.as_str() {
            "contract_executed" => {
                if let Some(c) = attrs.get("contract_id") {
                    have_contract.insert(c.clone());
                }
            }
            "order_created" => {
                if let Some(c) = attrs.get("contract_id") {
                    have_order.insert(c.clone());
                }
            }
            "invoice_issued" => {
                if let Some(c) = attrs.get("contract_id") {
                    if !have_contract.contains(c) || !have_order.contains(c) {
                        return false;
                    }
                }
            }
            _ => {}
        }
    }
    true
}

/// True if every contract that has a `partner_attributed` event has its
/// `partner_registered` event STRICTLY BEFORE the `contract_executed`.
pub fn partner_attribution_is_in_order(
    events: &[(String, std::collections::HashMap<String, String>)],
) -> bool {
    let mut contract_executed_idx: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut partner_registered_idx: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (i, (etype, attrs)) in events.iter().enumerate() {
        match etype.as_str() {
            "contract_executed" => {
                if let Some(c) = attrs.get("contract_id") {
                    contract_executed_idx.insert(c.clone(), i);
                }
            }
            "partner_registered" => {
                if let Some(p) = attrs.get("partner_id") {
                    partner_registered_idx.insert(p.clone(), i);
                }
            }
            _ => {}
        }
    }
    for (etype, attrs) in events {
        if etype == "partner_attributed" {
            if let (Some(p), Some(c)) = (attrs.get("partner_id"), attrs.get("contract_id")) {
                let p_idx = partner_registered_idx.get(p).copied().unwrap_or(usize::MAX);
                let c_idx = contract_executed_idx.get(c).copied().unwrap_or(0);
                if p_idx >= c_idx {
                    return false;
                }
            }
        }
    }
    true
}

/// Extract events from the OCEL store for a given session as a flat
/// (event_type, attrs) list.
pub fn observed_events(
    store: &OcelStore,
    session: &str,
) -> Vec<(String, std::collections::HashMap<String, String>)> {
    let log = store
        .build_ocel(Some(session))
        .expect("build_ocel should succeed");
    let mut sorted = log.events.clone();
    sorted.sort_by_key(|e| e.time);
    sorted
        .iter()
        .map(|e| {
            let mut attrs = std::collections::HashMap::new();
            for a in &e.attributes {
                attrs.insert(a.name.clone(), format!("{}", a.value));
            }
            (e.event_type.clone(), attrs)
        })
        .collect()
}
