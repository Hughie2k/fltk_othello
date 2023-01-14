pub mod play {
    use fltk::{
        self,
        app::{self, event_button, event_coords},
        button::{Button, RadioRoundButton},
        draw,
        enums::{Color, Event, FrameType},
        frame::Frame,
        group,
        group::ColorChooser,
        menu::Choice,
        output::Output,
        prelude::*,
        surface::ImageSurface,
        window::Window,
    };
    // use no_deadlocks::Mutex; // This can be interchanged with `std::sync::Mutex`, but is useful for debugging deadlocks
    // use no_deadlocks::Mutex;
    use othello::{board::*, evaluation};
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{mpsc, Arc, Mutex},
        thread,
    };

    const SIDE_LEN: i32 = 800;

    #[derive(Copy, Clone, Debug)]
    pub struct Colorscheme {
        board: (Color, Color),
        black: Color,
        white: Color,
        black_move: Color,
        white_move: Color,
    }

    impl Default for Colorscheme {
        fn default() -> Self {
            Colorscheme {
                board: (Color::from_hex(0x3a911au32), Color::from_hex(0x4ba30bu32)),
                black: Color::Black,
                white: Color::White,
                black_move: Color::from_hex(0x3f9e9bu32),
                white_move: Color::from_hex(0x3f9e9bu32),
            }
        }
    }

    fn draw_board(
        x: i32,
        y: i32,
        side_len: i32,
        piece_radius: i32,
        board: &Board,
        colorscheme: Colorscheme,
    ) {
        let moves = board.each_move();
        let sqsize = side_len / 8;
        let (moving_colour, waiting_colour, move_colour) = match board.black_moving {
            true => (colorscheme.black, colorscheme.white, colorscheme.black_move),
            false => (colorscheme.white, colorscheme.black, colorscheme.white_move),
        };

        for i in 0..8 {
            for j in 0..8 {
                draw::draw_rect_fill(
                    i * sqsize + x,
                    j * sqsize + y,
                    sqsize,
                    sqsize,
                    if (i + j) % 2 == 0 {
                        colorscheme.board.0
                    } else {
                        colorscheme.board.1
                    },
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
                    move_colour
                });
                // We have chosen the colour already, so now draw the circle
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

    pub fn main(app: app::App, human_black: bool, depth: u8, colorscheme: Colorscheme) {
        let (tx, rx) = mpsc::channel::<u64>();
        let board = Arc::new(Mutex::new(Board::default()));
        //let app = app::App::default();
        let mut wind = Window::new(700, 40, 1000, 1000, "Othello");

        let mut to_move = Output::new((wind.width() - 100) / 2, 15, 100, 20, "");
        to_move.set_frame(FrameType::FlatBox);

        let (mut blacks, mut whites) = (
            Output::new((wind.width() - 100) / 4, 15, 50, 20, "Blacks:"),
            Output::new((wind.width() * 3 - 100) / 4, 15, 50, 20, "Whites:"),
        );
        blacks.set_frame(FrameType::FlatBox);
        whites.set_frame(FrameType::FlatBox);

        let mut frame = Frame::new(
            (wind.width() - SIDE_LEN) / 2,
            (wind.height() - SIDE_LEN) / 2,
            SIDE_LEN,
            SIDE_LEN,
            "",
        );
        frame.set_color(Color::Blue);
        let surf = Arc::new(ImageSurface::new(frame.width(), frame.height(), false));
        let piece_radius = Rc::new(Cell::new(SIDE_LEN * 3 / 64));
        frame.draw({
            let piece_radius = piece_radius.clone();
            let board = board.clone();
            let surf = surf.clone();
            move |fr| {
                ImageSurface::push_current(&surf);
                draw_board(
                    0,
                    0,
                    SIDE_LEN,
                    piece_radius.get(),
                    &board.lock().unwrap(),
                    colorscheme.clone(),
                );
                ImageSurface::pop_current();
                surf.image().unwrap().draw(fr.x(), fr.y(), fr.w(), fr.h());
            }
        });
        frame.handle({
            let board = board.clone();
            move |frame, event| match event {
                Event::Push => {
                    let board = board.lock().unwrap().clone();
                    if event_button() == 1 && human_black == board.black_moving {
                        let (x, y) = event_coords();
                        let (x, y) = (
                            std::cmp::min(8 * (x - frame.x()) / SIDE_LEN, 7),
                            std::cmp::min(8 * (y - frame.y()) / SIDE_LEN, 7),
                        );
                        let move_bit = 1 << (8 * x + y);
                        tx.send(move_bit).expect("Reciever hung up");
                    };
                    true
                }
                _ => false,
            }
        });
        let frame = Arc::new(Mutex::new(frame));
        let mut button = Button::new(
            wind.width() / 2 - 100,
            SIDE_LEN + 30 + (*frame).lock().unwrap().y(),
            200,
            30,
            "Change piece size",
        );
        button.set_callback({
            let piece_radius = piece_radius.clone();
            let frame = frame.clone();
            move |_but| {
                piece_radius.set(SIDE_LEN * 5 / 64 - piece_radius.get());
                app::lock().expect("Locking unsupported");
                let mut frame = frame.lock().unwrap();
                frame.redraw();
                app::unlock();
                drop(frame);
            }
        });

        thread::spawn({
            let frame = frame.clone();
            let board = board.clone();
            move || {
                // let mut updates: Vec<Box<dyn FnMut(&mut Output)>> = vec![
                //     Box::new(|whites| {
                //         whites.set_value(&match false || board.lock().unwrap().black_moving {
                //             true => board.lock().unwrap().waiting.count().to_string(),
                //             false => board.lock().unwrap().to_move.count().to_string(),
                //         });
                //     }),
                //     Box::new(|blacks| {
                //         blacks.set_value(&match false || board.lock().unwrap().black_moving {
                //             false => board.lock().unwrap().waiting.count().to_string(),
                //             true => board.lock().unwrap().to_move.count().to_string(),
                //         })
                //     }),
                //     Box::new(|to_move| {
                //         to_move.set_value(&match false || board.lock().unwrap().black_moving {
                //             true => "Black's turn",
                //             false => "White's turn",
                //         })
                //     }),
                // ];
                let update = |whites: &mut Output,
                              blacks: &mut Output,
                              to_move: &mut Output,
                              board: &Board| match board.black_moving {
                    true => {
                        whites.set_value(&board.waiting.count().to_string());
                        blacks.set_value(&board.to_move.count().to_string());
                        to_move.set_value("Black's turn");
                    }
                    false => {
                        blacks.set_value(&board.waiting.count().to_string());
                        whites.set_value(&board.to_move.count().to_string());
                        to_move.set_value("White's turn");
                    }
                };
                loop {
                    let mut board_clone = board.lock().unwrap().clone();
                    while board_clone.black_moving != human_black
                        && board_clone.board_state == BoardState::Ongoing
                    {
                        // updates
                        //     .iter_mut()
                        //     .zip([&mut whites, &mut blacks, &mut to_move])
                        //     .for_each(|(f, k)| f(k));
                        update(&mut whites, &mut blacks, &mut to_move, &board_clone);
                        let move_bit =
                            evaluation::best_move(evaluation::better_eval, &board_clone, depth);
                        board_clone.make_move(move_bit);
                        board.lock().unwrap().make_move(move_bit);

                        app::lock().expect("Locking unsupported");

                        let mut frame = frame.lock().unwrap();
                        frame.redraw();
                        update(&mut whites, &mut blacks, &mut to_move, &board_clone);
                        // updates
                        //     .iter_mut()
                        //     .zip([&mut whites, &mut blacks, &mut to_move])
                        //     .for_each(|(f, k)| f(k));
                        app::unlock();

                        app::awake();
                    }
                    update(&mut whites, &mut blacks, &mut to_move, &board_clone);
                    // updates
                    //     .iter_mut()
                    //     .zip([&mut blacks, &mut whites, &mut to_move])
                    //     .for_each(|(f, k)| f(k));
                    let board_state = board.lock().unwrap().board_state;
                    match board_state {
                        BoardState::Ongoing => {
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
        let mut wind = Window::new(300, 300, 500, 800, "heheheha");
        let mut but = Button::new(100, 100, 300, 100, "MAKE A BOARD");
        let mut choices = group::Group::new(250, 300, 100, 50, Some("You play:"));
        let colorscheme = Rc::new(Cell::new(Colorscheme::default()));
        choices.set_align(fltk::enums::Align::Left);
        choices.set_label_size(20);
        let mut choice = RadioRoundButton::new(250, 300, 100, 25, "Black");
        choice.toggle(true);
        choices.add(&choice);
        choices.add(&RadioRoundButton::new(250, 325, 100, 25, "White"));
        choices.end();

        let mut depth = Choice::new(150, 420, 200, 50, "Minimax depth");
        (1..10).for_each(|n| depth.add_choice(&n.to_string()));
        depth.set_value(4);

        but.set_callback({
            let app = app.clone();
            let colorscheme = colorscheme.clone();
            move |_| {
                // dbg!("got c");
                main(
                    app.clone(),
                    choice.value(),
                    depth.value() as u8 + 1u8,
                    colorscheme.get(),
                    //colourscheme_setup(app.clone()),
                    //Colorscheme::default(),
                );
                // dbg!("running main");
                app::awake();
                app::check();
            }
        });
        but.set_color(Color::Green);

        let mut colortrig = Button::new(100, 500, 300, 100, "Set colours");
        colortrig.set_callback({
            let app = app.clone();
            let colorscheme = colorscheme.clone();
            move |_| colourscheme_setup(app.clone(), colorscheme.clone())
        });

        wind.end();
        wind.show();
        app.run().unwrap();
    }

    fn colourscheme_setup(app: app::App, colorscheme: Rc<Cell<Colorscheme>>) {
        let mut wind = Window::new(300, 300, 500, 800, "colours");
        let mut boardcolours = (
            ColorChooser::new(50, 50, 150, 150, "Board colour 1"),
            ColorChooser::new(300, 50, 150, 150, "Board colour 2"),
        );
        let mut disccolours = (
            ColorChooser::new(50, 300, 150, 150, "Black disc colour"),
            ColorChooser::new(300, 300, 150, 150, "White disc colour"),
        );
        boardcolours
            .0
            .set_tuple_rgb(colorscheme.get().board.0.to_rgb());
        boardcolours
            .1
            .set_tuple_rgb(colorscheme.get().board.1.to_rgb());
        disccolours
            .0
            .set_tuple_rgb(colorscheme.get().black.to_rgb());
        disccolours
            .1
            .set_tuple_rgb(colorscheme.get().white.to_rgb());

        let mut apply = Button::new(100, 550, 300, 200, "APPLY");
        apply.set_callback({
            let colorscheme = colorscheme.clone();
            move |_| {
                let next = Colorscheme {
                    board: (
                        Color::from_hex(boardcolours.0.hex_color()),
                        Color::from_hex(boardcolours.1.hex_color()),
                    ),
                    black: Color::from_hex(disccolours.0.hex_color()),
                    white: Color::from_hex(disccolours.1.hex_color()),
                    black_move: colorscheme.get().black_move,
                    white_move: colorscheme.get().white_move,
                };
                colorscheme.set(next);
            }
        });
        wind.end();
        wind.show();
        app.run().unwrap();
    }
}
