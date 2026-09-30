use memoria_memory::{
    replay_trace, transition_record, BatchRecoveryBoundaryCommand as Command,
    BatchRecoveryBoundaryEffect as Effect, BatchRecoveryBoundaryEffectResult as Fact,
    BatchRecoveryBoundaryInput as Input, BatchRecoveryBoundaryState as State, GitFact, JournalFact,
    SnapshotFact,
};

fn replay(
    command: Command,
    facts: Vec<Fact>,
) -> Vec<memoria_memory::BatchRecoveryBoundaryTransitionRecord> {
    replay_trace(
        State::Ready,
        std::iter::once(Input::Command(command)).chain(facts.into_iter().map(Input::EffectResult)),
    )
}

#[test]
fn every_public_operation_acquires_the_repository_lock_first() {
    let commands = [
        Command::Init,
        Command::Open,
        Command::Read,
        Command::Write,
        Command::Delete,
        Command::Exists,
        Command::List,
        Command::ReadMetadata,
        Command::Apply,
        Command::BuildContext,
        Command::Tree,
        Command::Status,
        Command::Diff,
        Command::Commit,
        Command::Log,
    ];
    for command in commands {
        let records = replay(command, vec![]);
        assert_eq!(records.len(), 1, "{command:?}");
        assert_eq!(records[0].state_after, State::AcquiringLock, "{command:?}");
        assert_eq!(
            records[0].output.effects,
            vec![Effect::AcquireRepoLock],
            "{command:?}"
        );
    }
}

#[test]
fn journal_durability_is_a_barrier_before_file_application() {
    let prefix = vec![
        Fact::LockAcquired,
        Fact::RepositoryObserved(true),
        Fact::JournalObserved(JournalFact::Empty),
        Fact::SnapshotObserved(SnapshotFact::MutationReady),
    ];
    let records = replay(Command::Apply, prefix.clone());
    assert_eq!(
        records.last().unwrap().output.effects,
        vec![Effect::PersistJournal]
    );
    assert!(records
        .iter()
        .all(|record| !record.output.effects.contains(&Effect::ApplyFilesystemPlan)));

    let mut uncertain = prefix.clone();
    uncertain.push(Fact::JournalPersistenceUncertain);
    let uncertain = replay(Command::Apply, uncertain);
    assert!(uncertain
        .iter()
        .all(|record| !record.output.effects.contains(&Effect::ApplyFilesystemPlan)));
    assert_eq!(
        uncertain.last().unwrap().output.effects,
        vec![Effect::ReleaseRepoLock]
    );

    let mut durable = prefix;
    durable.push(Fact::JournalDurable);
    let durable = replay(Command::Apply, durable);
    assert_eq!(
        durable.last().unwrap().output.effects,
        vec![Effect::ApplyFilesystemPlan]
    );
}

#[test]
fn uncertain_publication_routes_to_reconciliation() {
    let records = replay(
        Command::Commit,
        vec![
            Fact::LockAcquired,
            Fact::RepositoryObserved(true),
            Fact::JournalObserved(JournalFact::Empty),
            Fact::SnapshotObserved(SnapshotFact::GitNeeded),
            Fact::GitObserved(GitFact::CommitReady),
            Fact::CommitPublicationUnknown,
        ],
    );
    assert_eq!(
        records.last().unwrap().state_after,
        State::ReconcilingCommit
    );
    assert_eq!(
        records.last().unwrap().output.effects,
        vec![Effect::ReconcileCommit]
    );
}

pub fn generate_phase_schedules() -> Vec<(State, Input)> {
    let states = [
        State::Ready,
        State::AcquiringLock,
        State::InspectingRepository,
        State::InspectingJournal,
        State::RollingBack,
        State::CleaningRecoveredJournal,
        State::ReadingSnapshot,
        State::PersistingJournal,
        State::ApplyingFilesystem,
        State::PersistingCompletion,
        State::CleaningCompletedJournal,
        State::InspectingGit,
        State::PublishingCommit,
        State::ReconcilingCommit,
        State::ReleasingSuccess,
        State::ReleasingError,
    ];
    let facts = [
        Fact::LockAcquired,
        Fact::LockUnavailable,
        Fact::RepositoryObserved(true),
        Fact::RepositoryObserved(false),
        Fact::JournalObserved(JournalFact::Empty),
        Fact::JournalObserved(JournalFact::Incomplete),
        Fact::JournalObserved(JournalFact::Complete),
        Fact::JournalObserved(JournalFact::Conflict),
        Fact::RollbackDurable(false),
        Fact::RollbackConflict,
        Fact::JournalCleanupDurable,
        Fact::SnapshotObserved(SnapshotFact::ReadReady),
        Fact::SnapshotObserved(SnapshotFact::MutationReady),
        Fact::SnapshotObserved(SnapshotFact::GitNeeded),
        Fact::JournalDurable,
        Fact::JournalPersistenceUncertain,
        Fact::FilesystemPlanApplied(false),
        Fact::FilesystemApplyInterrupted,
        Fact::CompletionDurable,
        Fact::CompletionPersistenceUncertain,
        Fact::GitObserved(GitFact::CommitReady),
        Fact::CommitPublished,
        Fact::CommitPublicationUnknown,
        Fact::PublicationReconciled(true),
        Fact::PublicationStillUnknown,
        Fact::LockReleased,
        Fact::LockReleaseError,
    ];
    states
        .into_iter()
        .flat_map(|state| {
            facts
                .into_iter()
                .map(move |fact| (state, Input::EffectResult(fact)))
        })
        .collect()
}

#[test]
fn check_phase_protocol() {
    for (state, input) in generate_phase_schedules() {
        let result = transition_record(state, input);
        assert!(result.output.effects.len() <= 1);
        assert!(!(result.output.reply.is_some() && result.output.rejection.is_some()));
        if result.output.reply.is_some() {
            assert_eq!(state, State::ReleasingSuccess);
            assert_eq!(result.state_after, State::Ready);
        }
        if result.output.effects.contains(&Effect::ApplyFilesystemPlan) {
            assert!(matches!(
                (state, input),
                (
                    State::PersistingJournal,
                    Input::EffectResult(Fact::JournalDurable)
                ) | (
                    State::ApplyingFilesystem,
                    Input::EffectResult(Fact::FilesystemPlanApplied(true))
                )
            ));
        }
    }
}

pub fn generate_provider_observations() -> Vec<(JournalFact, State, Effect)> {
    let outcomes = [
        (
            JournalFact::Empty,
            State::ReadingSnapshot,
            Effect::ReadSnapshot,
        ),
        (
            JournalFact::Incomplete,
            State::RollingBack,
            Effect::RollbackJournal,
        ),
        (
            JournalFact::Complete,
            State::CleaningRecoveredJournal,
            Effect::CleanupJournal,
        ),
        (
            JournalFact::Conflict,
            State::ReleasingError,
            Effect::ReleaseRepoLock,
        ),
    ];
    (0..128)
        .map(|seed| outcomes[(seed * 37 + 3) % outcomes.len()])
        .collect()
}

#[test]
fn check_provider_routing() {
    for (fact, expected_state, expected_effect) in generate_provider_observations() {
        let record = transition_record(
            State::InspectingJournal,
            Input::EffectResult(Fact::JournalObserved(fact)),
        );
        assert_eq!(record.state_after, expected_state);
        assert_eq!(record.output.effects, vec![expected_effect]);
    }
}

pub fn generate_filesystem_topologies() -> Vec<(State, Fact)> {
    let failures = [
        (State::PersistingJournal, Fact::JournalPersistenceUncertain),
        (State::ApplyingFilesystem, Fact::FilesystemApplyInterrupted),
        (
            State::PersistingCompletion,
            Fact::CompletionPersistenceUncertain,
        ),
        (State::RollingBack, Fact::RollbackConflict),
    ];
    (0..128)
        .map(|seed| failures[(seed * 41 + 1) % failures.len()])
        .collect()
}

#[test]
fn check_filesystem_safety() {
    for (state, fact) in generate_filesystem_topologies() {
        let result = transition_record(state, Input::EffectResult(fact));
        assert!(result.output.reply.is_none());
        assert!(!result.output.effects.contains(&Effect::ApplyFilesystemPlan));
        assert!(!result.output.effects.contains(&Effect::PersistCompletion));
    }
}

pub fn generate_crash_schedules() -> Vec<Vec<Fact>> {
    let prefix = vec![Fact::LockAcquired, Fact::RepositoryObserved(true)];
    [
        JournalFact::Empty,
        JournalFact::Incomplete,
        JournalFact::Complete,
        JournalFact::Conflict,
    ]
    .into_iter()
    .map(|fact| {
        let mut schedule = prefix.clone();
        schedule.push(Fact::JournalObserved(fact));
        schedule
    })
    .collect()
}

#[test]
fn check_crash_recovery() {
    for facts in generate_crash_schedules() {
        let records = replay(Command::Open, facts);
        assert!(records.iter().all(|r| r.output.reply.is_none()));
        let last = records.last().unwrap();
        assert_ne!(last.state_after, State::ApplyingFilesystem);
        assert_ne!(last.state_after, State::PersistingCompletion);
    }
}

pub fn generate_git_schedules() -> Vec<Fact> {
    let outcomes = [
        Fact::CommitPublished,
        Fact::CommitNotPublished,
        Fact::CommitPublicationUnknown,
    ];
    (0..129)
        .map(|seed| outcomes[(seed * 17 + 2) % outcomes.len()])
        .collect()
}

#[test]
fn check_git_publication() {
    for fact in generate_git_schedules() {
        let result = transition_record(State::PublishingCommit, Input::EffectResult(fact));
        assert_ne!(result.output.effects, vec![Effect::PublishCommit]);
        if fact == Fact::CommitPublicationUnknown {
            assert_eq!(result.output.effects, vec![Effect::ReconcileCommit]);
            assert_eq!(result.state_after, State::ReconcilingCommit);
        }
    }
}

pub fn generate_native_operations() -> Vec<Command> {
    let operations = [
        Command::Init,
        Command::Open,
        Command::Read,
        Command::Write,
        Command::Delete,
        Command::Exists,
        Command::List,
        Command::ReadMetadata,
        Command::Apply,
        Command::BuildContext,
        Command::Tree,
        Command::Status,
        Command::Diff,
        Command::Commit,
        Command::Log,
    ];
    (0..150)
        .map(|seed| operations[(seed * 31 + 1) % operations.len()])
        .collect()
}

#[test]
fn check_public_operations() {
    for command in generate_native_operations() {
        let result = transition_record(State::Ready, Input::Command(command));
        assert_eq!(result.output.effects, vec![Effect::AcquireRepoLock]);
        assert_eq!(result.state_after, State::AcquiringLock);
    }
    let invalid = transition_record(State::Ready, Input::Command(Command::InvalidRequestInput));
    assert!(invalid.output.effects.is_empty());
    assert!(invalid.output.rejection.is_some());
}

pub fn generate_requests() -> Vec<String> {
    let mut paths = vec![
        "../escape.md".into(),
        "/etc/passwd".into(),
        "knowledge/.git/config.md".into(),
        "system/identity.md".into(),
    ];
    for index in 0..128 {
        paths.push(format!("knowledge/generated-{index}.md"));
    }
    paths
}

#[test]
fn check_request_validation() {
    let temp = tempfile::tempdir().unwrap();
    let repo = memoria_memory::MemoryRepo::init(temp.path().join("memory")).unwrap();
    for path in generate_requests() {
        let result = repo.read(&path);
        if path.contains("..") || path.starts_with('/') || path.contains(".git") {
            assert_eq!(result.unwrap_err().code(), "invalid_request", "{path}");
        } else {
            assert!(
                matches!(result.unwrap_err().code(), "not_found" | "invalid_request"),
                "{path}"
            );
        }
    }
}
