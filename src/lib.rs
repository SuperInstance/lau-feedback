use serde::{Deserialize, Serialize};

/// Signals that describe how a kid is responding to the tutor/game.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FeedbackSignal {
    SkippedHint,
    CompletedQuickly(f64),
    AskedForHelp,
    GaveUp,
    Celebrated,
    TriedAgainAfterFailure,
    ExploredOnOwn(String),
    TaughtPeer(String),
}

/// A single feedback observation at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEvent {
    pub tick: u64,
    pub player: String,
    pub signal: FeedbackSignal,
    pub context: String,
}

/// Collects feedback events and supports queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeedbackCollector {
    events: Vec<FeedbackEvent>,
}

impl FeedbackCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, player: &str, signal: FeedbackSignal, context: &str, tick: u64) {
        self.events.push(FeedbackEvent {
            tick,
            player: player.to_string(),
            signal,
            context: context.to_string(),
        });
    }

    pub fn events_for(&self, player: &str) -> Vec<&FeedbackEvent> {
        self.events
            .iter()
            .filter(|e| e.player == player)
            .collect()
    }

    pub fn recent_signals(&self, ticks: u64) -> Vec<&FeedbackEvent> {
        let cutoff = self.events.last().map_or(0, |e| e.tick.saturating_sub(ticks));
        self.events
            .iter()
            .filter(|e| e.tick >= cutoff)
            .collect()
    }

    pub fn signal_count(&self, signal: &FeedbackSignal) -> usize {
        self.events.iter().filter(|e| &e.signal == signal).count()
    }

    pub fn all_events(&self) -> &[FeedbackEvent] {
        &self.events
    }
}

/// Multi-dimensional engagement score. All fields 0.0–1.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementScore {
    pub focus: f64,
    pub persistence: f64,
    pub curiosity: f64,
    pub frustration: f64,
    pub joy: f64,
}

impl EngagementScore {
    pub fn new() -> Self {
        Self {
            focus: 0.0,
            persistence: 0.0,
            curiosity: 0.0,
            frustration: 0.0,
            joy: 0.0,
        }
    }

    /// Weighted overall engagement score.
    pub fn overall(&self) -> f64 {
        (self.focus * 0.3
            + self.persistence * 0.2
            + self.curiosity * 0.3
            + self.joy * 0.2
            - self.frustration * 0.3)
            .clamp(0.0, 1.0)
    }

    pub fn is_engaged(&self) -> bool {
        self.overall() > 0.5
    }

    pub fn is_frustrated(&self) -> bool {
        self.frustration > 0.7
    }
}

impl Default for EngagementScore {
    fn default() -> Self {
        Self::new()
    }
}

/// Analyzes a slice of events and produces an engagement score.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EngagementAnalyzer;

impl EngagementAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(&self, events: &[FeedbackEvent]) -> EngagementScore {
        let mut score = EngagementScore::new();
        if events.is_empty() {
            return score;
        }

        let n = events.len() as f64;
        let mut focus_sum = 0.0_f64;
        let mut persistence_sum = 0.0_f64;
        let mut curiosity_sum = 0.0_f64;
        let mut frustration_sum = 0.0_f64;
        let mut joy_sum = 0.0_f64;

        for event in events {
            match &event.signal {
                FeedbackSignal::CompletedQuickly(_) => {
                    focus_sum += 1.0;
                }
                FeedbackSignal::SkippedHint => {
                    focus_sum += 0.5;
                }
                FeedbackSignal::AskedForHelp => {
                    persistence_sum += 0.8;
                }
                FeedbackSignal::TriedAgainAfterFailure => {
                    persistence_sum += 1.0;
                }
                FeedbackSignal::GaveUp => {
                    frustration_sum += 1.0;
                }
                FeedbackSignal::Celebrated => {
                    joy_sum += 1.0;
                }
                FeedbackSignal::ExploredOnOwn(_) => {
                    curiosity_sum += 1.0;
                }
                FeedbackSignal::TaughtPeer(_) => {
                    curiosity_sum += 0.8;
                    joy_sum += 0.8;
                }
            }
        }

        score.focus = (focus_sum / n).clamp(0.0, 1.0);
        score.persistence = (persistence_sum / n).clamp(0.0, 1.0);
        score.curiosity = (curiosity_sum / n).clamp(0.0, 1.0);
        score.frustration = (frustration_sum / n).clamp(0.0, 1.0);
        score.joy = (joy_sum / n).clamp(0.0, 1.0);

        score
    }
}

/// Adjusts difficulty based on engagement feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyAdjustment {
    pub current: f64,
    pub target_engagement: f64,
}

impl DifficultyAdjustment {
    pub fn new() -> Self {
        Self {
            current: 0.5,
            target_engagement: 0.7,
        }
    }

    /// Returns the new difficulty level after adjusting based on engagement.
    pub fn adjust(&mut self, engagement: &EngagementScore) -> f64 {
        let overall = engagement.overall();

        if engagement.is_frustrated() {
            self.current = (self.current - 0.1).clamp(0.0, 1.0);
        } else if overall > self.target_engagement + 0.1 {
            // Too engaged → make it harder
            self.current = (self.current + 0.05).clamp(0.0, 1.0);
        } else if overall < self.target_engagement - 0.1 {
            // Under-engaged → easier
            self.current = (self.current - 0.05).clamp(0.0, 1.0);
        }
        // else: in the zone, stay

        self.current
    }
}

impl Default for DifficultyAdjustment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- FeedbackSignal tests ---

    #[test]
    fn signal_serde_roundtrip() {
        let signal = FeedbackSignal::ExploredOnOwn("puzzle".into());
        let json = serde_json::to_string(&signal).unwrap();
        let back: FeedbackSignal = serde_json::from_str(&json).unwrap();
        assert_eq!(signal, back);
    }

    #[test]
    fn all_signal_variants_serde() {
        let signals = vec![
            FeedbackSignal::SkippedHint,
            FeedbackSignal::CompletedQuickly(3.2),
            FeedbackSignal::AskedForHelp,
            FeedbackSignal::GaveUp,
            FeedbackSignal::Celebrated,
            FeedbackSignal::TriedAgainAfterFailure,
            FeedbackSignal::ExploredOnOwn("math".into()),
            FeedbackSignal::TaughtPeer("sorting".into()),
        ];
        for s in &signals {
            let json = serde_json::to_string(s).unwrap();
            let back: FeedbackSignal = serde_json::from_str(&json).unwrap();
            assert_eq!(s, &back);
        }
    }

    // --- FeedbackCollector tests ---

    #[test]
    fn record_and_events_for() {
        let mut c = FeedbackCollector::new();
        c.record("alice", FeedbackSignal::Celebrated, "level 1", 1);
        c.record("bob", FeedbackSignal::GaveUp, "level 2", 2);
        c.record("alice", FeedbackSignal::AskedForHelp, "level 3", 3);

        assert_eq!(c.events_for("alice").len(), 2);
        assert_eq!(c.events_for("bob").len(), 1);
        assert_eq!(c.events_for("carol").len(), 0);
    }

    #[test]
    fn recent_signals() {
        let mut c = FeedbackCollector::new();
        c.record("a", FeedbackSignal::Celebrated, "", 10);
        c.record("a", FeedbackSignal::GaveUp, "", 20);
        c.record("a", FeedbackSignal::Celebrated, "", 30);

        let recent = c.recent_signals(15);
        assert_eq!(recent.len(), 2); // ticks 20 and 30 are within 15 of 30
    }

    #[test]
    fn recent_signals_empty() {
        let c = FeedbackCollector::new();
        assert!(c.recent_signals(100).is_empty());
    }

    #[test]
    fn signal_count_by_variant() {
        let mut c = FeedbackCollector::new();
        c.record("a", FeedbackSignal::Celebrated, "", 1);
        c.record("a", FeedbackSignal::Celebrated, "", 2);
        c.record("a", FeedbackSignal::GaveUp, "", 3);

        assert_eq!(c.signal_count(&FeedbackSignal::Celebrated), 2);
        assert_eq!(c.signal_count(&FeedbackSignal::GaveUp), 1);
        assert_eq!(c.signal_count(&FeedbackSignal::AskedForHelp), 0);
    }

    #[test]
    fn signal_count_with_data() {
        let mut c = FeedbackCollector::new();
        c.record("a", FeedbackSignal::CompletedQuickly(1.0), "", 1);
        c.record("a", FeedbackSignal::CompletedQuickly(2.0), "", 2);

        // Different inner values still match the variant
        assert_eq!(c.signal_count(&FeedbackSignal::CompletedQuickly(1.0)), 1);
        assert_eq!(c.signal_count(&FeedbackSignal::CompletedQuickly(2.0)), 1);
    }

    #[test]
    fn collector_serde_roundtrip() {
        let mut c = FeedbackCollector::new();
        c.record("x", FeedbackSignal::TaughtPeer("loops".into()), "context", 5);
        let json = serde_json::to_string(&c).unwrap();
        let back: FeedbackCollector = serde_json::from_str(&json).unwrap();
        assert_eq!(back.all_events().len(), 1);
    }

    // --- EngagementScore tests ---

    #[test]
    fn engagement_score_default_not_engaged() {
        let s = EngagementScore::new();
        assert!(!s.is_engaged());
        assert!(!s.is_frustrated());
    }

    #[test]
    fn engagement_score_overall_perfect() {
        let s = EngagementScore {
            focus: 1.0,
            persistence: 1.0,
            curiosity: 1.0,
            frustration: 0.0,
            joy: 1.0,
        };
        // 0.3 + 0.2 + 0.3 + 0.2 - 0 = 1.0
        assert!((s.overall() - 1.0).abs() < 1e-9);
        assert!(s.is_engaged());
    }

    #[test]
    fn engagement_score_frustration_drag() {
        let s = EngagementScore {
            focus: 1.0,
            persistence: 1.0,
            curiosity: 1.0,
            frustration: 1.0,
            joy: 1.0,
        };
        // 0.3 + 0.2 + 0.3 + 0.2 - 0.3 = 0.7
        assert!((s.overall() - 0.7).abs() < 1e-9);
    }

    #[test]
    fn engagement_score_clamped_at_zero() {
        let s = EngagementScore {
            focus: 0.0,
            persistence: 0.0,
            curiosity: 0.0,
            frustration: 1.0,
            joy: 0.0,
        };
        assert_eq!(s.overall(), 0.0);
    }

    #[test]
    fn is_frustrated() {
        let s = EngagementScore {
            frustration: 0.8,
            ..EngagementScore::new()
        };
        assert!(s.is_frustrated());
    }

    #[test]
    fn not_frustrated_below_threshold() {
        let s = EngagementScore {
            frustration: 0.69,
            ..EngagementScore::new()
        };
        assert!(!s.is_frustrated());
    }

    #[test]
    fn engagement_score_serde() {
        let s = EngagementScore {
            focus: 0.5,
            persistence: 0.6,
            curiosity: 0.7,
            frustration: 0.1,
            joy: 0.9,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: EngagementScore = serde_json::from_str(&json).unwrap();
        assert!((back.focus - 0.5).abs() < 1e-9);
    }

    // --- EngagementAnalyzer tests ---

    #[test]
    fn analyze_empty() {
        let a = EngagementAnalyzer::new();
        let s = a.analyze(&[]);
        assert!(!s.is_engaged());
    }

    #[test]
    fn analyze_completed_quickly_boosts_focus() {
        let a = EngagementAnalyzer::new();
        let events = vec![FeedbackEvent {
            tick: 1,
            player: "a".into(),
            signal: FeedbackSignal::CompletedQuickly(2.0),
            context: "".into(),
        }];
        let s = a.analyze(&events);
        assert_eq!(s.focus, 1.0);
    }

    #[test]
    fn analyze_mixed_signals() {
        let a = EngagementAnalyzer::new();
        let events = vec![
            FeedbackEvent {
                tick: 1,
                player: "a".into(),
                signal: FeedbackSignal::Celebrated,
                context: "".into(),
            },
            FeedbackEvent {
                tick: 2,
                player: "a".into(),
                signal: FeedbackSignal::GaveUp,
                context: "".into(),
            },
        ];
        let s = a.analyze(&events);
        assert_eq!(s.joy, 0.5);
        assert_eq!(s.frustration, 0.5);
    }

    #[test]
    fn analyze_taught_peer_boosts_curiosity_and_joy() {
        let a = EngagementAnalyzer::new();
        let events = vec![FeedbackEvent {
            tick: 1,
            player: "a".into(),
            signal: FeedbackSignal::TaughtPeer("variables".into()),
            context: "".into(),
        }];
        let s = a.analyze(&events);
        assert!((s.curiosity - 0.8).abs() < 1e-9);
        assert!((s.joy - 0.8).abs() < 1e-9);
    }

    #[test]
    fn analyze_persistence_signals() {
        let a = EngagementAnalyzer::new();
        let events = vec![
            FeedbackEvent {
                tick: 1,
                player: "a".into(),
                signal: FeedbackSignal::AskedForHelp,
                context: "".into(),
            },
            FeedbackEvent {
                tick: 2,
                player: "a".into(),
                signal: FeedbackSignal::TriedAgainAfterFailure,
                context: "".into(),
            },
        ];
        let s = a.analyze(&events);
        // (0.8 + 1.0) / 2 = 0.9
        assert!((s.persistence - 0.9).abs() < 1e-9);
    }

    #[test]
    fn analyze_skipped_hint_partial_focus() {
        let a = EngagementAnalyzer::new();
        let events = vec![FeedbackEvent {
            tick: 1,
            player: "a".into(),
            signal: FeedbackSignal::SkippedHint,
            context: "".into(),
        }];
        let s = a.analyze(&events);
        assert!((s.focus - 0.5).abs() < 1e-9);
    }

    // --- DifficultyAdjustment tests ---

    #[test]
    fn difficulty_default() {
        let d = DifficultyAdjustment::new();
        assert!((d.current - 0.5).abs() < 1e-9);
        assert!((d.target_engagement - 0.7).abs() < 1e-9);
    }

    #[test]
    fn difficulty_stays_in_zone() {
        let mut d = DifficultyAdjustment::new();
        let engagement = EngagementScore {
            focus: 0.7,
            persistence: 0.7,
            curiosity: 0.7,
            frustration: 0.0,
            joy: 0.7,
        };
        let before = d.current;
        let new = d.adjust(&engagement);
        assert!((new - before).abs() < 1e-9);
    }

    #[test]
    fn difficulty_increases_when_too_engaged() {
        let mut d = DifficultyAdjustment::new();
        let engagement = EngagementScore {
            focus: 1.0,
            persistence: 1.0,
            curiosity: 1.0,
            frustration: 0.0,
            joy: 1.0,
        };
        let new = d.adjust(&engagement);
        assert!(new > 0.5);
    }

    #[test]
    fn difficulty_decreases_when_frustrated() {
        let mut d = DifficultyAdjustment::new();
        let engagement = EngagementScore {
            focus: 0.0,
            persistence: 0.0,
            curiosity: 0.0,
            frustration: 0.9,
            joy: 0.0,
        };
        let new = d.adjust(&engagement);
        assert!(new < 0.5);
    }

    #[test]
    fn difficulty_decreases_when_under_engaged() {
        let mut d = DifficultyAdjustment::new();
        let engagement = EngagementScore {
            focus: 0.1,
            persistence: 0.1,
            curiosity: 0.1,
            frustration: 0.0,
            joy: 0.1,
        };
        let new = d.adjust(&engagement);
        assert!(new < 0.5);
    }

    #[test]
    fn difficulty_clamped_at_one() {
        let mut d = DifficultyAdjustment {
            current: 0.99,
            target_engagement: 0.7,
        };
        let engagement = EngagementScore {
            focus: 1.0,
            persistence: 1.0,
            curiosity: 1.0,
            frustration: 0.0,
            joy: 1.0,
        };
        let new = d.adjust(&engagement);
        assert!(new <= 1.0);
    }

    #[test]
    fn difficulty_clamped_at_zero() {
        let mut d = DifficultyAdjustment {
            current: 0.01,
            target_engagement: 0.7,
        };
        let engagement = EngagementScore {
            frustration: 0.9,
            ..EngagementScore::new()
        };
        let new = d.adjust(&engagement);
        assert!(new >= 0.0);
    }

    #[test]
    fn difficulty_serde() {
        let d = DifficultyAdjustment::new();
        let json = serde_json::to_string(&d).unwrap();
        let back: DifficultyAdjustment = serde_json::from_str(&json).unwrap();
        assert!((back.current - 0.5).abs() < 1e-9);
    }
}
