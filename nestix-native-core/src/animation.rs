use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use nestix::{Shared, State, create_state, untrack};

use crate::{Length, ResolvedStyle, TransitionProperty, WithAuto};

/// Timing curve used by an animation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    #[default]
    EaseInOut,
}

impl Easing {
    fn sample(self, value: f64) -> f64 {
        match self {
            Self::Linear => value,
            Self::EaseIn => value * value,
            Self::EaseOut => 1.0 - (1.0 - value) * (1.0 - value),
            Self::EaseInOut if value < 0.5 => 2.0 * value * value,
            Self::EaseInOut => 1.0 - (-2.0 * value + 2.0).powi(2) / 2.0,
        }
    }
}

/// Duration and timing curve applied to target changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationSpec {
    pub duration: Duration,
    pub easing: Easing,
}

impl AnimationSpec {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            easing: Easing::default(),
        }
    }

    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}

#[derive(Clone)]
struct AnimationControl {
    running: Rc<Cell<bool>>,
    settle: Shared<dyn Fn()>,
}

#[derive(Default)]
struct HandleInner {
    controls: RefCell<Vec<AnimationControl>>,
}

/// Controls the animations created by an imperative transaction.
#[derive(Clone, Default)]
pub struct AnimationHandle {
    inner: Rc<HandleInner>,
}

impl AnimationHandle {
    pub fn is_running(&self) -> bool {
        self.inner
            .controls
            .borrow()
            .iter()
            .any(|control| control.running.get())
    }

    /// Stops the group and applies all final target values.
    pub fn cancel(&self) {
        self.settle();
    }

    /// Completes the group immediately at its final target values.
    pub fn finish(&self) {
        self.settle();
    }

    fn settle(&self) {
        let controls = self.inner.controls.borrow().clone();
        for control in controls {
            (control.settle)();
        }
    }
}

struct Transaction {
    spec: AnimationSpec,
    handle: AnimationHandle,
}

struct TransactionGuard(Option<Transaction>);

impl Drop for TransactionGuard {
    fn drop(&mut self) {
        TRANSACTION.with(|current| {
            current.replace(self.0.take());
        });
    }
}

thread_local! {
    static TRANSACTION: RefCell<Option<Transaction>> = const { RefCell::new(None) };
}

/// Applies an animation specification to synchronous target changes in `update`.
pub fn animate(spec: AnimationSpec, update: impl FnOnce()) -> AnimationHandle {
    let handle = AnimationHandle::default();
    let previous = TRANSACTION.with(|current| {
        current.replace(Some(Transaction {
            spec,
            handle: handle.clone(),
        }))
    });
    let _guard = TransactionGuard(previous);
    update();
    handle
}

fn transaction() -> Option<(AnimationSpec, AnimationHandle)> {
    TRANSACTION.with(|current| {
        current
            .borrow()
            .as_ref()
            .map(|transaction| (transaction.spec, transaction.handle.clone()))
    })
}

struct ActiveAnimation {
    started: Instant,
    spec: AnimationSpec,
    from: f64,
    to: f64,
    running: Rc<Cell<bool>>,
    update: Shared<dyn Fn(f64)>,
}

/// Shared UI-thread animation engine. Backends call [`Self::tick`] from their frame source.
pub struct AnimationRuntime {
    active: RefCell<HashMap<(u64, TransitionProperty), ActiveAnimation>>,
    clock: Shared<dyn Fn() -> Instant>,
}

impl Default for AnimationRuntime {
    fn default() -> Self {
        let clock: Rc<dyn Fn() -> Instant> = Rc::new(Instant::now);
        Self {
            active: RefCell::new(HashMap::new()),
            clock: Shared::from(clock),
        }
    }
}

impl AnimationRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    #[doc(hidden)]
    pub fn with_clock(clock: Shared<dyn Fn() -> Instant>) -> Self {
        Self {
            active: RefCell::new(HashMap::new()),
            clock,
        }
    }

    pub fn is_active(&self) -> bool {
        !self.active.borrow().is_empty()
    }

    fn transition(
        self: &Rc<Self>,
        key: (u64, TransitionProperty),
        from: f64,
        to: f64,
        spec: AnimationSpec,
        handle: Option<AnimationHandle>,
        update: Shared<dyn Fn(f64)>,
    ) {
        self.cancel(key);
        if spec.duration.is_zero() || from == to {
            update(to);
            return;
        }

        let running = Rc::new(Cell::new(true));
        let weak_runtime = Rc::downgrade(self);
        let settle_update = update.clone();
        let settle_running = running.clone();
        let settle: Rc<dyn Fn()> = Rc::new(move || {
            if !settle_running.replace(false) {
                return;
            }
            if let Some(runtime) = weak_runtime.upgrade() {
                runtime.active.borrow_mut().remove(&key);
            }
            settle_update(to);
        });
        let settle = Shared::from(settle);
        if let Some(handle) = handle {
            handle.inner.controls.borrow_mut().push(AnimationControl {
                running: running.clone(),
                settle,
            });
        }

        self.active.borrow_mut().insert(
            key,
            ActiveAnimation {
                started: (self.clock)(),
                spec,
                from,
                to,
                running,
                update,
            },
        );
    }

    pub fn cancel_owner(&self, owner: u64) {
        let keys = self
            .active
            .borrow()
            .keys()
            .filter(|(candidate, _)| *candidate == owner)
            .copied()
            .collect::<Vec<_>>();
        for key in keys {
            self.cancel(key);
        }
    }

    fn cancel(&self, key: (u64, TransitionProperty)) {
        if let Some(animation) = self.active.borrow_mut().remove(&key) {
            animation.running.set(false);
        }
    }

    /// Samples all active animations. Returns whether more frames are needed.
    pub fn tick(&self) -> bool {
        self.tick_at((self.clock)())
    }

    fn tick_at(&self, now: Instant) -> bool {
        let keys = self.active.borrow().keys().copied().collect::<Vec<_>>();
        for key in keys {
            let finished = {
                let active = self.active.borrow();
                let Some(animation) = active.get(&key) else {
                    continue;
                };
                let elapsed = now.saturating_duration_since(animation.started);
                let linear =
                    (elapsed.as_secs_f64() / animation.spec.duration.as_secs_f64()).clamp(0.0, 1.0);
                let value = animation.from
                    + (animation.to - animation.from) * animation.spec.easing.sample(linear);
                (animation.update)(value);
                linear >= 1.0
            };
            if finished && let Some(animation) = self.active.borrow_mut().remove(&key) {
                animation.running.set(false);
            }
        }
        self.is_active()
    }
}

/// Presentation copy of a resolved style whose geometry follows an animation runtime.
pub struct AnimatedStyle {
    owner: u64,
    runtime: Rc<AnimationRuntime>,
    value: State<Option<ResolvedStyle>>,
}

impl AnimatedStyle {
    pub fn new(runtime: Rc<AnimationRuntime>, initial: Option<ResolvedStyle>) -> Self {
        static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);
        Self {
            owner: NEXT_OWNER.fetch_add(1, Ordering::Relaxed),
            runtime,
            value: create_state(initial),
        }
    }

    pub fn value(&self) -> State<Option<ResolvedStyle>> {
        self.value.clone()
    }

    pub fn set_target(&self, target: Option<ResolvedStyle>, scale_factor: f64) {
        let Some(target) = target else {
            self.runtime.cancel_owner(self.owner);
            self.value.set(None);
            return;
        };
        // Presentation updates are written by the frame clock. Reading them
        // must not make the target-watching effect depend on its own output.
        let current = untrack(|| self.value.get()).unwrap_or_default();
        let transaction = transaction();
        let mut presentation = target.clone();

        for property in TransitionProperty::GEOMETRY {
            let from = property.length_with_auto(&current);
            let to = property.length_with_auto(&target);
            let spec = transaction
                .as_ref()
                .map(|(spec, _)| *spec)
                .or_else(|| target.transition_for(property));
            let (Some(spec), Some(from), Some(to)) = (spec, from, to) else {
                self.runtime.cancel((self.owner, property));
                continue;
            };
            let (Some(from), Some(to)) = (
                logical_length_with_auto(from, scale_factor),
                logical_length_with_auto(to, scale_factor),
            ) else {
                self.runtime.cancel((self.owner, property));
                continue;
            };
            property.set_length_with_auto(&mut presentation, WithAuto::from(from));
            let state = self.value.clone();
            let update: Rc<dyn Fn(f64)> = Rc::new(move |value| {
                state.mutate(|style| {
                    if let Some(style) = style {
                        property.set_length_with_auto(style, WithAuto::from(value));
                    }
                });
            });
            let update = Shared::from(update);
            self.runtime.transition(
                (self.owner, property),
                from,
                to,
                spec,
                transaction.as_ref().map(|(_, handle)| handle.clone()),
                update,
            );
        }

        self.value.set(Some(presentation));
    }
}

impl Drop for AnimatedStyle {
    fn drop(&mut self) {
        self.runtime.cancel_owner(self.owner);
    }
}

fn logical_length_with_auto(value: WithAuto<Length>, scale_factor: f64) -> Option<f64> {
    match value {
        WithAuto::Auto => None,
        WithAuto::Value(value) => Some(value.to_logical::<f64>(scale_factor).0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StyleTransition;
    use nestix::effect;

    #[test]
    fn easing_reaches_endpoints() {
        for easing in [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
        ] {
            assert_eq!(easing.sample(0.0), 0.0);
            assert_eq!(easing.sample(1.0), 1.0);
        }
    }

    #[test]
    fn zero_duration_applies_immediately() {
        let runtime = Rc::new(AnimationRuntime::new());
        let value = create_state(0.0);
        let update: Rc<dyn Fn(f64)> = Rc::new({
            let value = value.clone();
            move |next| value.set(next)
        });
        runtime.transition(
            (1, TransitionProperty::Width),
            0.0,
            10.0,
            AnimationSpec::new(Duration::ZERO),
            None,
            Shared::from(update),
        );
        assert_eq!(value.get(), 10.0);
        assert!(!runtime.is_active());
    }

    #[test]
    fn fake_clock_drives_eased_progress_and_completion() {
        let origin = Instant::now();
        let elapsed = Rc::new(Cell::new(Duration::ZERO));
        let clock: Rc<dyn Fn() -> Instant> = Rc::new({
            let elapsed = elapsed.clone();
            move || origin + elapsed.get()
        });
        let runtime = Rc::new(AnimationRuntime::with_clock(Shared::from(clock)));
        let value = create_state(0.0);
        let update: Rc<dyn Fn(f64)> = Rc::new({
            let value = value.clone();
            move |next| value.set(next)
        });
        runtime.transition(
            (1, TransitionProperty::Width),
            0.0,
            10.0,
            AnimationSpec::new(Duration::from_millis(100)).easing(Easing::Linear),
            None,
            Shared::from(update),
        );

        elapsed.set(Duration::from_millis(50));
        assert!(runtime.tick());
        assert_eq!(value.get(), 5.0);
        elapsed.set(Duration::from_millis(100));
        assert!(!runtime.tick());
        assert_eq!(value.get(), 10.0);
    }

    #[test]
    fn transaction_is_restored_after_panic() {
        let result = std::panic::catch_unwind(|| {
            animate(AnimationSpec::new(Duration::from_millis(100)), || {
                panic!("test panic");
            });
        });
        assert!(result.is_err());
        assert!(transaction().is_none());
    }

    #[test]
    fn presentation_ticks_do_not_retrigger_target_effects() {
        let origin = Instant::now();
        let elapsed = Rc::new(Cell::new(Duration::ZERO));
        let clock: Rc<dyn Fn() -> Instant> = Rc::new({
            let elapsed = elapsed.clone();
            move || origin + elapsed.get()
        });
        let runtime = Rc::new(AnimationRuntime::with_clock(Shared::from(clock)));
        let transition = StyleTransition {
            property: TransitionProperty::Width,
            animation: AnimationSpec::new(Duration::from_millis(100)),
        };
        let mut initial = ResolvedStyle::default();
        initial.width = Some(WithAuto::from(0));
        initial.transitions = vec![transition];
        let target = create_state(Some(initial));
        let animated = Rc::new(AnimatedStyle::new(runtime.clone(), target.get()));
        let runs = Rc::new(Cell::new(0));
        let effect_handle = effect({
            let animated = animated.clone();
            let target = target.clone();
            let runs = runs.clone();
            move || {
                runs.set(runs.get() + 1);
                animated.set_target(target.get(), 1.0);
            }
        });

        let mut next = ResolvedStyle::default();
        next.width = Some(WithAuto::from(10));
        next.transitions = vec![transition];
        target.set(Some(next));
        assert_eq!(runs.get(), 2);
        elapsed.set(Duration::from_millis(50));
        runtime.tick();
        assert_eq!(runs.get(), 2);
        assert_eq!(
            animated.value().get().and_then(|style| style.width),
            Some(WithAuto::from(5)),
        );
        effect_handle.cancel();
    }
}
