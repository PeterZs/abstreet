mod setup;

use crate::game::{GameState, Mode};
use crate::state::PerMapUI;
use crate::ui::ShowEverything;
use abstutil::elapsed_seconds;
use ezgui::{Color, EventCtx, EventLoopMode, GfxCtx, Text, Wizard};
use geom::Duration;
use sim::{Benchmark, Sim};
use std::collections::HashMap;
use std::time::Instant;

const ADJUST_SPEED: f64 = 0.1;

pub struct ABTestMode {
    pub desired_speed: f64, // sim seconds per real second
    pub state: State,
    // TODO Urgh, hack. Need to be able to take() it to switch states sometimes.
    pub secondary: Option<PerMapUI>,
}

pub enum State {
    Setup(setup::ABTestSetup),
    Paused,
    Running {
        last_step: Instant,
        benchmark: Benchmark,
        speed: String,
    },
}

impl ABTestMode {
    pub fn new() -> ABTestMode {
        ABTestMode {
            desired_speed: 1.0,
            state: State::Setup(setup::ABTestSetup::Pick(Wizard::new())),
            secondary: None,
        }
    }

    pub fn event(state: &mut GameState, ctx: &mut EventCtx) -> EventLoopMode {
        match state.mode {
            Mode::ABTest(ref mut mode) => {
                if let State::Setup(_) = mode.state {
                    setup::ABTestSetup::event(state, ctx);
                    return EventLoopMode::InputOnly;
                }

                ctx.canvas.handle_event(ctx.input);
                state.ui.state.primary.current_selection = state.ui.handle_mouseover(
                    ctx,
                    None,
                    &state.ui.state.primary.sim,
                    &ShowEverything::new(),
                    false,
                );

                let mut txt = Text::new();
                txt.add_styled_line("A/B Test Mode".to_string(), None, Some(Color::BLUE), None);
                txt.add_line(state.ui.state.primary.map.get_edits().edits_name.clone());
                txt.add_line(state.ui.state.primary.sim.summary());
                if let State::Running { ref speed, .. } = mode.state {
                    txt.add_line(format!(
                        "Speed: {0} / desired {1:.2}x",
                        speed, mode.desired_speed
                    ));
                } else {
                    txt.add_line(format!(
                        "Speed: paused / desired {0:.2}x",
                        mode.desired_speed
                    ));
                }
                ctx.input
                    .set_mode_with_new_prompt("A/B Test Mode", txt, ctx.canvas);

                if ctx.input.modal_action("quit") {
                    // TODO This shouldn't be necessary when we plumb state around instead of
                    // sharing it in the old structure.
                    state.ui.state.primary.sim = Sim::new(
                        &state.ui.state.primary.map,
                        state
                            .ui
                            .state
                            .primary
                            .current_flags
                            .sim_flags
                            .run_name
                            .clone(),
                        None,
                    );
                    state.mode = Mode::SplashScreen(Wizard::new(), None);
                    return EventLoopMode::InputOnly;
                }

                if ctx.input.modal_action("slow down sim") {
                    mode.desired_speed -= ADJUST_SPEED;
                    mode.desired_speed = mode.desired_speed.max(0.0);
                }
                if ctx.input.modal_action("speed up sim") {
                    mode.desired_speed += ADJUST_SPEED;
                }
                if ctx.input.modal_action("swap") {
                    let secondary = mode.secondary.take().unwrap();
                    let primary = std::mem::replace(&mut state.ui.state.primary, secondary);
                    mode.secondary = Some(primary);
                }

                match mode.state {
                    State::Paused => {
                        if ctx.input.modal_action("run/pause sim") {
                            mode.state = State::Running {
                                last_step: Instant::now(),
                                benchmark: state.ui.state.primary.sim.start_benchmark(),
                                speed: "...".to_string(),
                            };
                        } else if ctx.input.modal_action("run one step of sim") {
                            state.ui.state.primary.sim.step(&state.ui.state.primary.map);
                            {
                                let s = mode.secondary.as_mut().unwrap();
                                s.sim.step(&s.map);
                            }
                            //*ctx.recalculate_current_selection = true;
                        }
                        EventLoopMode::InputOnly
                    }
                    State::Running {
                        ref mut last_step,
                        ref mut benchmark,
                        ref mut speed,
                    } => {
                        if ctx.input.modal_action("run/pause sim") {
                            mode.state = State::Paused;
                        } else if ctx.input.nonblocking_is_update_event() {
                            // TODO https://gafferongames.com/post/fix_your_timestep/
                            // TODO This doesn't interact correctly with the fixed 30 Update events sent
                            // per second. Even Benchmark is kind of wrong. I think we want to count the
                            // number of steps we've done in the last second, then stop if the speed says
                            // we should.
                            let dt_s = elapsed_seconds(*last_step);
                            if dt_s >= sim::TIMESTEP.inner_seconds() / mode.desired_speed {
                                ctx.input.use_update_event();
                                state.ui.state.primary.sim.step(&state.ui.state.primary.map);
                                {
                                    let s = mode.secondary.as_mut().unwrap();
                                    s.sim.step(&s.map);
                                }
                                //*ctx.recalculate_current_selection = true;
                                *last_step = Instant::now();

                                if benchmark.has_real_time_passed(Duration::seconds(1.0)) {
                                    // I think the benchmark should naturally account for the delay of
                                    // the secondary sim.
                                    *speed =
                                        state.ui.state.primary.sim.measure_speed(benchmark, false);
                                }
                            }
                        }
                        EventLoopMode::Animation
                    }
                    State::Setup(_) => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn draw(state: &GameState, g: &mut GfxCtx) {
        state.ui.new_draw(
            g,
            None,
            HashMap::new(),
            &state.ui.state.primary.sim,
            &ShowEverything::new(),
        );

        match state.mode {
            Mode::ABTest(ref mode) => match mode.state {
                State::Setup(ref setup) => {
                    setup.draw(g);
                }
                _ => {}
            },
            _ => unreachable!(),
        }
    }
}
