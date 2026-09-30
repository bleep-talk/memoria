use batch_recovery_decisions::*;
use std::collections::BTreeMap;
fn generate_histories() -> Vec<(usize, usize)> {
    (1..9)
        .flat_map(|size| (0..size + 4).map(move |fault| (size, fault)))
        .collect()
}
fn contents(batch: &PreparedBatch, after: bool) -> BTreeMap<MemoryPath, Option<ContentHash>> {
    batch
        .changes()
        .iter()
        .map(|c| {
            (
                c.path().clone(),
                if after {
                    c.after_hash()
                } else {
                    c.before_hash()
                },
            )
        })
        .collect()
}
#[test]
fn recovery_property() {
    for (size, fault) in generate_histories() {
        let changes = (0..size)
            .map(|i| {
                FileChange::new(
                    MemoryPath::new(&format!("knowledge/{i}.md")).unwrap(),
                    Some(format!("before-{i}")),
                    Some(format!("after-{i}")),
                )
            })
            .collect();
        let batch =
            PreparedBatch::new(format!("batch-{size}-{fault}"), changes, &Policy::default())
                .unwrap();
        let mut current = contents(&batch, false);
        let mut pending = PendingBatch::new(batch.clone());
        let mut marker = CompletionMarker::Absent;
        let mut plan = pending.plan().unwrap();
        let mut tick = 0;
        let mut did_fault = false;
        loop {
            let action = plan.action().clone();
            let result = if tick == fault && !did_fault {
                did_fault = true;
                ObservationResult::Failed
            } else {
                ObservationResult::Confirmed
            };
            let mut cleaned = false;
            if result == ObservationResult::Confirmed {
                match action {
                    PlanAction::Apply(i) => {
                        let c = &batch.changes()[i];
                        current.insert(c.path().clone(), c.after_hash());
                    }
                    PlanAction::Rollback(i) => {
                        let c = &batch.changes()[i];
                        current.insert(c.path().clone(), c.before_hash());
                    }
                    PlanAction::MarkCompletion => marker = CompletionMarker::DurableComplete,
                    PlanAction::Cleanup => cleaned = true,
                    _ => {}
                }
            }
            let o = RecoveryObservation::new(
                batch.id().into(),
                batch.digest().clone(),
                plan.sequence(),
                action,
                marker.clone(),
                result,
                current.clone(),
                true,
                cleaned,
            );
            let decision = pending.observe(o.clone()).unwrap();
            match decision {
                RecoveryDecision::Continue(next, next_plan)
                | RecoveryDecision::Inspect(next, next_plan) => {
                    let duplicate = next.observe(o).unwrap();
                    assert!(matches!(duplicate, RecoveryDecision::Duplicate(_)));
                    pending = next;
                    plan = next_plan;
                }
                RecoveryDecision::Settled(settled) => {
                    let applied = settled.outcome() == &BatchOutcome::Applied;
                    assert_eq!(current, contents(&batch, applied));
                    assert_eq!(applied, marker == CompletionMarker::DurableComplete);
                    break;
                }
                RecoveryDecision::Duplicate(_) => panic!("unexpected duplicate"),
            }
            tick += 1;
            assert!(
                tick < 64,
                "non-terminating recovery for size={size} fault={fault}"
            );
        }
        // Every interruption point recovers from durable contents, independent of remembered phase.
        for applied in 0..=size {
            let mut observed = contents(&batch, false);
            for c in batch.changes().iter().take(applied) {
                observed.insert(c.path().clone(), c.after_hash());
            }
            let observation = RecoveryObservation::new(
                batch.id().into(),
                batch.digest().clone(),
                0,
                PlanAction::Inspect,
                CompletionMarker::Absent,
                ObservationResult::Confirmed,
                observed,
                true,
                false,
            );
            assert!(
                matches!(PendingBatch::new(batch.clone()).resume(observation),Ok(RecoveryDecision::Continue(_,plan)) if matches!(plan.action(),PlanAction::Rollback(0)))
            );
        }
        let mut foreign = contents(&batch, false);
        foreign.insert(
            batch.changes()[0].path().clone(),
            Some(hash_content(b"external edit")),
        );
        let o = RecoveryObservation::new(
            batch.id().into(),
            batch.digest().clone(),
            0,
            PlanAction::Inspect,
            CompletionMarker::Absent,
            ObservationResult::Confirmed,
            foreign,
            true,
            false,
        );
        assert!(matches!(
            PendingBatch::new(batch.clone()).resume(o),
            Err(MemoryError::UnexpectedRecoveryContents(_))
        ));
    }
}
