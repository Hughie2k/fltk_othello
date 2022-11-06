use fltk::{
    self,
    app::{self, event_button, event_coords},
    button::Button,
    draw,
    enums::{Color, Event},
    frame::Frame,
    prelude::*,
    surface::ImageSurface,
    window::Window,
};
use othello::{board::*, evaluation};
use std::{
    borrow::Borrow,
    cell::Cell,
    rc::Rc,
    sync::{mpsc, Arc, Mutex},
    thread,
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

fn main() {
    let (tx, rx) = mpsc::channel::<u64>();
    let board = Arc::new(Mutex::new(Board::default()));
    let app = app::App::default();
    let mut wind = Window::new(700, 400, 600, 600, "HI:)");
    let mut frame = Frame::new(50, 50, SIDE_LEN, SIDE_LEN, "title");
    let human_black = Arc::new(Cell::new(true));
    frame.set_color(Color::Blue);
    let piece_radius = Arc::new(Cell::new(20));
    let surf = Arc::new(ImageSurface::new(frame.width(), frame.height(), false));
    frame.draw({
        let piece_radius = piece_radius.clone();
        let board = board.clone();
        let surf = surf.clone();
        move |fr| {
            ImageSurface::push_current(&surf);
            draw_board(0, 0, SIDE_LEN, piece_radius.get(), &board.lock().unwrap());
            ImageSurface::pop_current();
            surf.image().unwrap().draw(fr.x(), fr.y(), fr.w(), fr.h());
            dbg!("{}", piece_radius.get());
        }
    });
    frame.handle({
        let tx = tx.clone();
        let board = board.clone();
        move |frame, event| match event {
            Event::Push => {
                let mut board = board.lock().unwrap().clone();
                if event_button() == 1 && human_black.get() == board.black_moving {
                    let (x, y) = event_coords();
                    let (x, y) = (
                        8 * (x - frame.x()) / SIDE_LEN,
                        8 * (y - frame.y()) / SIDE_LEN,
                    );
                    let move_bit = 1 << (8 * x + y); // TODO: Make sure (8 * x + y) < 64
                    tx.send(move_bit);
                    board.safe_make_move(move_bit);
                    // board.safe_make_move(move_bit);
                    // println!("x, y = {x}, {y}");
                    // println!("{:?}", (*board).borrow());
                    // frame.redraw();

                    thread::spawn({
                        let mut board = board.clone();
                        let human_black = human_black.get();
                        let tx = tx.clone();
                        move || {
                            while board.black_moving != human_black {
                                let move_bit =
                                    evaluation::best_move(evaluation::better_eval, &board, 9);
                                tx.send(move_bit).expect("Failed send");
                                board.make_move(move_bit);
                            }
                        }
                    });
                    // This part should be transferred out of the main thread. We must not lock the mutex in that thread because that thread could take a while to finish
                    // let (tx, rx) = std::sync::mpsc::channel::<u64>();
                    // let rx = Rc::new(rx);
                    // thread::spawn({
                    //     let mut board = board.get();
                    //     let human_black = human_black.get();
                    //     move || {
                    //         while board.black_moving != human_black {
                    //             let move_bit =
                    //                 evaluation::best_move(evaluation::better_eval, &board, 6);
                    //             tx.send(move_bit).expect("Failed to send");
                    //             board.make_move(move_bit);
                    //         }
                    //     }
                    // });
                };
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
            frame.lock().unwrap().redraw();
        }
    });
    thread::spawn({
        let frame = frame.clone();
        let board = board.clone();
        move || loop {
            let move_bit = rx.recv().unwrap();
            board.lock().unwrap().safe_make_move(move_bit);
            frame.lock().unwrap().redraw();
        }
    });
    wind.end();
    wind.show();
    app.run().unwrap();
}
