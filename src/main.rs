use std::{
  error::Error,
  io, thread,
  time::{Duration, Instant},
};

use crossterm::{
  cursor::{Hide, Show},
  event::{self, Event, KeyCode},
  terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
  ExecutableCommand,
};
use invaders::{
  frame::{self, new_frame, Drawable, Frame},
  invaders::Invaders,
  player::Player,
  render,
};
use rusty_audio::Audio;

fn main() -> Result<(), Box<dyn Error>> {
  let mut audio = Audio::new();
  audio.add("explode", "sounds/explode.wav");
  audio.add("lose", "sounds/lose.wav");
  audio.add("move", "sounds/move.wav");
  audio.add("pew", "sounds/pew.wav");
  audio.add("startup", "sounds/startup.wav");
  audio.add("win", "sounds/win.wav");
  audio.play("startup");

  //terminal
  let mut stdio = io::stdout();
  terminal::enable_raw_mode().unwrap();
  stdio.execute(EnterAlternateScreen).unwrap();
  stdio.execute(Hide).unwrap();

  //Render loop in a seperate thread
  let (render_tx, render_rx) = std::sync::mpsc::channel::<Frame>();
  let render_handle = thread::spawn(move || {
    let mut last_frame = frame::new_frame();
    let mut stdout = io::stdout();
    render::render(&mut stdout, &last_frame, &last_frame, true);
    loop {
      let curr_frame = match render_rx.recv() {
        Ok(x) => x,
        Err(_) => break,
      };

      render::render(&mut stdout, &last_frame, &curr_frame, false);
      last_frame = curr_frame;
    }
  });

  let mut player = Player::new();
  let mut instant = Instant::now();
  let mut invaders = Invaders::new();

  //Game loop
  'gameloop: loop {
    //Per frame init
    let delta = instant.elapsed();
    instant = Instant::now();
    let mut cur_frame = new_frame();

    //Input
    while event::poll(Duration::default()).unwrap() {
      if let Event::Key(key) = event::read().unwrap() {
        match key.code {
          KeyCode::Left => player.move_left(),
          KeyCode::Right => player.move_right(),
          KeyCode::Char(' ') => {
            if player.shoot() {
              audio.play("pew");
            }
          }
          KeyCode::Esc | KeyCode::Char('q') => {
            audio.play("lose");
            break 'gameloop;
          }
          _ => {}
        }
      }
    }

    //Update
    player.update(delta);
    if invaders.update(delta) {
      audio.play("move");
    }
    if player.detect_hits(&mut invaders) {
      audio.play("explode");
    }

    //Render
    let drawables: Vec<&dyn Drawable> = vec![&player, &invaders];
    for drawable in &drawables {
      drawable.draw(&mut cur_frame);
    }
    render_tx.send(cur_frame).unwrap();
    thread::sleep(Duration::from_millis(1));

    // Win condition
    if invaders.all_killed() {
      audio.play("win");
      break 'gameloop;
    }

    if invaders.reached_bottom() {
      audio.play("lose");
      break 'gameloop;
    }
  }

  //Cleanup
  drop(render_tx);
  render_handle.join().unwrap();
  audio.wait();
  stdio.execute(Show).unwrap();
  stdio.execute(LeaveAlternateScreen).unwrap();
  terminal::disable_raw_mode().unwrap();
  Ok(())
}
