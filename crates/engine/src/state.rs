//! The one place that says which job state transitions are legal.

use crate::model::JobState::{self, *};

pub fn can_transition(from: JobState, to: JobState) -> bool {
    matches!(
        (from, to),
        // Queued → Failed covers "a tool is missing" before anything runs.
        (Queued, Probing | Downloading | Paused | Canceled | Failed)
            // → Queued from a running state is the quit-and-resume path.
            | (Probing, Downloading | Failed | Paused | Canceled | Queued)
            // Downloading → Probing is the re-probe after an HTTP 403 on cached info.
            | (
                Downloading,
                PostProcessing | Completed | Failed | Paused | Canceled | Queued | Probing
            )
            // yt-dlp runs some post-processors before the download, so this goes both ways.
            | (
                PostProcessing,
                Downloading | Completed | Failed | Paused | Canceled | Queued
            )
            | (Paused, Queued | Canceled)
            | (Failed, Queued | Canceled)
            | (Canceled, Queued)
    )
}

/// A yt-dlp process is (or is about to be) running for this job.
pub fn is_active(state: JobState) -> bool {
    matches!(state, Probing | Downloading | PostProcessing)
}

/// Nothing more will happen to this job unless the user acts.
pub fn is_finished(state: JobState) -> bool {
    matches!(state, Completed | Failed | Canceled)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [JobState; 8] = [Queued, Probing, Downloading, PostProcessing, Completed, Paused, Failed, Canceled];

    #[test]
    fn transition_table() {
        // Rows: from. Columns: to, in the order of ALL. 1 = legal.
        let table: [(JobState, [u8; 8]); 8] = [
            (Queued, [0, 1, 1, 0, 0, 1, 1, 1]),
            (Probing, [1, 0, 1, 0, 0, 1, 1, 1]),
            (Downloading, [1, 1, 0, 1, 1, 1, 1, 1]),
            (PostProcessing, [1, 0, 1, 0, 1, 1, 1, 1]),
            (Completed, [0, 0, 0, 0, 0, 0, 0, 0]),
            (Paused, [1, 0, 0, 0, 0, 0, 0, 1]),
            (Failed, [1, 0, 0, 0, 0, 0, 0, 1]),
            (Canceled, [1, 0, 0, 0, 0, 0, 0, 0]),
        ];
        for (from, row) in table {
            for (to, legal) in ALL.iter().zip(row) {
                assert_eq!(can_transition(from, *to), legal == 1, "{from:?} -> {to:?}");
            }
        }
    }

    #[test]
    fn completed_is_terminal_and_nothing_loops_to_itself() {
        for s in ALL {
            assert!(!can_transition(Completed, s));
            assert!(!can_transition(s, s));
        }
    }
}
