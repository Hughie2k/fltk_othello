pub mod play {
    use fltk::{
        self,
        app::{self, event_button, event_coords, flush},
        button::{Button, RadioButton, RadioRoundButton},
        draw,
        enums::{Color, Event, FrameType},
        frame::Frame,
        group,
        output::Output,
        prelude::*,
        surface::ImageSurface,
        text::TextDisplay,
        window::Window,
    };
    // use no_deadlocks::Mutex; // This can be interchanged with `std::sync::Mutex`, but is useful for debugging deadlocks
    use no_deadlocks::Mutex;
    use othello::{board::*, evaluation};
    use std::{
        cell::Cell,
        collections::HashMap,
        hash::Hash,
        sync::{mpsc, Arc /*Mutex*/},
        thread, time,
    };

    const SIDE_LEN: i32 = 500;

    fn draw_board(x: i32, y: i32, side_len: i32, piece_radius: i32, board: &Board) {
        let moves = board.each_move();
        let sqsize = side_len / 8;
        let (moving_colour, waiting_colour) = match board.black_moving {
            true => (Color::Black, Color::White),
            false => (Color::White, Color::Black),
        };

        for i in 0..8 {
            for j in 0..8 {
                draw::draw_rect_fill(
                    i * sqsize + x,
                    j * sqsize + y,
                    sqsize,
                    sqsize,
                    Color::from_hex(0x3a914au32 + ((i + j) as u32 % 2u32) * 0x111111u32),
                );
                let cur_bit = 1 << (i * 8 + j);

                // Check if we should show a piece or possible move on this square
                if (board.to_move.bits | board.waiting.bits | moves.bits) & cur_bit == 0 {
                    continue;
                }
                draw::set_draw_color(if board.to_move.bits & cur_bit != 0 {
                    moving_colour
                } else if board.waiting.bits & cur_bit != 0 {
                    waiting_colour
                } else {
                    Color::from_hex(0x3f9e9bu32)
                });
                draw::draw_pie(
                    i * sqsize + sqsize / 2 - piece_radius + x,
                    j * sqsize + sqsize / 2 - piece_radius + y,
                    2 * piece_radius,
                    2 * piece_radius,
                    0.0,
                    360.0,
                );
            }
        }

        draw::set_draw_color(Color::Black);
        for i in 0..=8 {
            draw::draw_line(i * sqsize + x, y, i * sqsize + x, 8 * sqsize + y);
            draw::draw_line(x, i * sqsize + y, 8 * sqsize + x, i * sqsize + y);
        }
    }

    pub fn main(app: app::App, human_black: bool) {
        let (tx, rx) = mpsc::channel::<u64>();
        let board = Arc::new(Mutex::new(Board::default()));
        //let app = app::App::default();
        let mut wind = Window::new(700, 400, 600, 600, "HI:)");

        let mut to_move = Output::new(250, 15, 100, 20, "");
        to_move.set_frame(FrameType::FlatBox);

        let (mut blacks, mut whites) = (
            Output::new(100, 15, 50, 20, "Blacks:"),
            Output::new(450, 15, 50, 20, "Whites:"),
        );
        blacks.set_frame(FrameType::FlatBox);
        whites.set_frame(FrameType::FlatBox);

        let mut frame = Frame::new(50, 50, SIDE_LEN, SIDE_LEN, "title");
        let human_black = Arc::new(human_black);
        frame.set_color(Color::Blue);
        let piece_radius = Arc::new(Cell::new(20));
        let surf = Arc::new(ImageSurface::new(frame.width(), frame.height(), false));
        frame.draw({
            let piece_radius = piece_radius.clone();
            let board = board.clone();
            let surf = surf.clone();
            move |fr| {
                ImageSurface::push_current(&surf);
                dbg!("Frame locking board");
                draw_board(0, 0, SIDE_LEN, piece_radius.get(), &board.lock().unwrap());
                dbg!("Frame done with board");
                ImageSurface::pop_current();
                surf.image().unwrap().draw(fr.x(), fr.y(), fr.w(), fr.h());
                dbg!("{}", piece_radius.get());
            }
        });
        frame.handle({
            let tx = tx.clone();
            let board = board.clone();
            let human_black = human_black.clone();
            move |frame, event| match event {
                Event::Push => {
                    dbg!("Handling frame");
                    let board = board.lock().unwrap().clone();
                    dbg!("locked board");
                    if event_button() == 1 && *human_black == board.black_moving {
                        let (x, y) = event_coords();
                        dbg!("working");
                        let (x, y) = (
                            std::cmp::min(8 * (x - frame.x()) / SIDE_LEN, 7),
                            std::cmp::min(8 * (y - frame.y()) / SIDE_LEN, 7),
                        );
                        let move_bit = 1 << (8 * x + y);
                        tx.send(move_bit).expect("Reciever hung up");
                        dbg!("sent move");
                    };
                    dbg!("Returning true");
                    true
                }
                _ => false,
            }
        });
        let frame = Arc::new(Mutex::new(frame));
        let mut button = Button::new(200, 560, 200, 30, "Change piece size");
        let piece_radius = piece_radius.clone();
        button.set_callback({
            let frame = frame.clone();
            move |_but| {
                piece_radius.set(30 - piece_radius.get());
                dbg!(piece_radius.get());
                dbg!("Button locking frame");
                app::lock().expect("Locking unsupported");
                let mut frame = frame.lock().unwrap();
                dbg!("Button frame locked");
                dbg!("Button frame redrawing");
                //thread::sleep(time::Duration::from_millis(500));
                frame.redraw();
                app::unlock();
                dbg!("Button frame redrawn");
                drop(frame);
            }
        });

        thread::spawn({
            let frame = frame.clone();
            let board = board.clone();
            move || {
                let mut updates: Vec<Box<dyn FnMut(&mut Output)>> = vec![
                    Box::new(|whites| {
                        whites.set_value(&match false || board.lock().unwrap().black_moving {
                            true => board.lock().unwrap().waiting.count().to_string(),
                            false => board.lock().unwrap().to_move.count().to_string(),
                        });
                    }),
                    Box::new(|blacks| {
                        blacks.set_value(&match false || board.lock().unwrap().black_moving {
                            false => board.lock().unwrap().waiting.count().to_string(),
                            true => board.lock().unwrap().to_move.count().to_string(),
                        })
                    }),
                    Box::new(|to_move| {
                        to_move.set_value(&match false || board.lock().unwrap().black_moving {
                            true => "Black's turn",
                            false => "White's turn",
                        })
                    }),
                ];
                loop {
                    let mut board_clone = board.lock().unwrap().clone();
                    while board_clone.black_moving != *human_black
                        && board_clone.board_state == BoardState::Ongoing
                    {
                        updates
                            .iter_mut()
                            .zip([&mut blacks, &mut whites, &mut to_move])
                            .for_each(|(f, k)| f(k));
                        let move_bit =
                            evaluation::best_move(evaluation::better_eval, &board_clone, 5);
                        board_clone.make_move(move_bit);
                        board.lock().unwrap().make_move(move_bit);

                        app::lock().expect("Locking unsupported");

                        let mut frame = frame.lock().unwrap();
                        frame.redraw();
                        updates
                            .iter_mut()
                            .zip([&mut blacks, &mut whites, &mut to_move])
                            .for_each(|(f, k)| f(k));
                        app::unlock();

                        app::awake();
                    }
                    updates
                        .iter_mut()
                        .zip([&mut blacks, &mut whites, &mut to_move])
                        .for_each(|(f, k)| f(k));
                    let board_state = board.lock().unwrap().board_state;
                    match board_state {
                        BoardState::Ongoing => {
                            match board.try_lock() {
                                Ok(_) => dbg!("Mutex not locked"),
                                Err(_) => dbg!("Mutex locked"),
                            };

                            dbg!("Waiting for move");
                            let move_bit = rx.recv().unwrap();
                            board.lock().unwrap().safe_make_move(move_bit);
                        }
                        BoardState::Drawn => {
                            to_move.set_value("Game ended in a draw.");
                            return;
                        }
                        BoardState::Won => {
                            let winning_colour = match board.lock().unwrap().black_moving {
                                true => "Black",
                                false => "White",
                            };
                            let msg = format!("{winning_colour} wins!");
                            to_move.set_value(&msg);
                            return;
                        }
                    }
                    app::lock().expect("Locking unsupported");
                    frame.lock().unwrap().redraw();

                    app::unlock();
                    app::awake();
                }
            }
        });
        wind.end();
        wind.show();
        app.run().unwrap();
    }

    pub fn board_setup() {
        let app = app::App::default();
        let mut wind = Window::new(300, 300, 500, 500, "heheheha");
        let mut but = Button::new(100, 100, 300, 100, "MAKE A BOARD");
        let mut choices = group::Group::new(250, 300, 100, 50, Some("You play:"));
        choices.set_align(fltk::enums::Align::Left);
        choices.set_label_size(20);
        let mut choice = RadioRoundButton::new(250, 300, 100, 25, "Black");
        choice.toggle(true);
        choices.add(&choice);
        choices.add(&RadioRoundButton::new(250, 325, 100, 25, "White"));

        but.set_callback({
            let app = app.clone();
            move |_| main(app.clone(), choice.value())
        });
        but.set_color(Color::Green);
        wind.end();
        wind.show();
        app.run().unwrap();
    }
}
