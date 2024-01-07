use std::{
    io::{stdout, Error, Result, Stdout, Write},
    thread,
    time::{Duration, Instant},
};

use crossterm::{
    cursor::MoveTo,
    event::{poll, read, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType},
    QueueableCommand,
};
use rand::Rng;
use term_color::BRIGHT_RED_BG;

use crate::term_color::{RST, WHITE_BG};

mod term_color;

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn get_delta(&self) -> (i32, i32) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }
}

struct Cell<'a> {
    x: u16,
    y: u16,
    content: &'a str,
    color: &'a str,
}

impl Cell<'_> {
    fn fruit(max_x: u16, max_y: u16) -> Cell<'static> {
        let mut x: u16 = (rand::thread_rng().gen::<f32>() * (max_x - 1) as f32) as u16;
        if x == 0 {
            x = 1
        }
        let mut y: u16 = (rand::thread_rng().gen::<f32>() * (max_y - 1) as f32) as u16;
        if y == 0 {
            y = 1
        }
        Cell {
            x,
            y,
            content: " ",
            color: BRIGHT_RED_BG,
        }
    }

    fn snake(x: u16, y: u16) -> Cell<'static> {
        Cell {
            x,
            y,
            content: " ",
            color: WHITE_BG,
        }
    }
}

struct Snake<'a> {
    direction: Direction,
    body: Vec<Cell<'a>>,
}

impl Snake<'_> {
    pub fn new() -> Snake<'static> {
        Snake {
            direction: Direction::Right,
            body: vec![Cell::snake(5, 5), Cell::snake(5, 4)],
        }
    }
}

const MS_PER_FRAME: u64 = 60; //1000 / 60;

fn main() -> Result<()> {
    println!("Snakeli");
    let game = Game::init()?;
    game.run()
}

struct Game<'a> {
    clock: Instant,
    stdout: Stdout,
    term_size: TermSize,
    snake: Snake<'a>,
    fruit: Cell<'a>,
    paused: bool,
    lost: bool,
}

struct TermSize {
    pub w: u16,
    pub h: u16,
}

impl Game<'_> {
    pub fn init() -> Result<Game<'static>> {
        let (w, h) = size()?;
        let (w, h) = (50u16, h / 2);
        Ok(Game {
            clock: Instant::now(),
            stdout: stdout(),
            term_size: TermSize { w, h },
            snake: Snake::new(),
            fruit: Cell::fruit(w, h),
            paused: false,
            lost: false,
        })
    }

    pub fn run(mut self) -> Result<()> {
        enable_raw_mode()?;
        self.stdout.queue(Clear(ClearType::All))?;
        self.stdout.flush()?;
        let mut frames: usize = 0;
        let _frames_start = Instant::now();
        self.render_border()?;

        loop {
            frames = frames + 1;
            // process input
            self.handle_event()?;
            // update game state
            self.update_snake()?;
            match self.handle_collision() {
                Err(err) if err.to_string() == "LOST" => {
                    break;
                }
                Err(err) => {
                    eprintln!("Error handling collision: {:?}", err);
                }
                _ => {}
            }
            // render

            self.clock = Instant::now();
            self.render()?;
            thread::sleep(Duration::from_millis(MS_PER_FRAME) - Instant::now().duration_since(self.clock));
        }
        if self.lost {
            self.render_lose()?;
        }
        Ok(())
    }

    fn render(&mut self) -> Result<()> {
        self.render_fruit()?;
        self.render_snake()?;
        self.stdout.flush()?;
        Ok(())
    }

    fn render_border(&mut self) -> Result<()> {
        self.stdout.queue(MoveTo(0, 0))?;
        self.stdout.write(WHITE_BG.as_bytes())?;
        self.stdout.write(" ".repeat(self.term_size.w.into()).as_bytes())?;
        self.stdout.write(RST.as_bytes())?;
        for y in 1..self.term_size.h {
            self.stdout.queue(MoveTo(0, y))?;
            self.stdout.write(WHITE_BG.as_bytes())?;
            self.stdout.write(" ".as_bytes())?;
            self.stdout.queue(MoveTo(self.term_size.w - 1, y))?;
            self.stdout.write(" ".as_bytes())?;
            self.stdout.write(RST.as_bytes())?;
        }
        self.stdout.queue(MoveTo(0, self.term_size.h))?;
        self.stdout.write(WHITE_BG.as_bytes())?;
        self.stdout.write(" ".repeat(self.term_size.w.into()).as_bytes())?;
        self.stdout.write(RST.as_bytes())?;
        Ok(())
    }

    fn handle_event(&mut self) -> Result<()> {
        if poll(Duration::from_millis((MS_PER_FRAME as f64 * 0.5) as u64))? {
            match read()? {
                Event::Key(event) => match event.code {
                    KeyCode::Up | KeyCode::Char('w') => match self.snake.direction {
                        Direction::Down => {}
                        _ => self.snake.direction = Direction::Up,
                    },
                    KeyCode::Down | KeyCode::Char('s') => match self.snake.direction {
                        Direction::Up => {}
                        _ => self.snake.direction = Direction::Down,
                    },
                    KeyCode::Left | KeyCode::Char('a') => match self.snake.direction {
                        Direction::Right => {}
                        _ => self.snake.direction = Direction::Left,
                    },
                    KeyCode::Right | KeyCode::Char('f') => match self.snake.direction {
                        Direction::Left => {}
                        _ => self.snake.direction = Direction::Right,
                    },
                    KeyCode::Char(' ') => {
                        disable_raw_mode()?;
                        self.stdout.flush()?;
                        self.paused = !self.paused;
                        todo!("Handle pause")
                    }
                    _ => {}
                },
                Event::Resize(_width, _height) => {
                    todo!("Handle screen resize")
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn update_snake(&mut self) -> Result<()> {
        let head = self.snake.body.first().unwrap();
        let (dx, dy) = self.snake.direction.get_delta();
        let new_head = Cell::snake((head.x as i32 + dx) as u16, (head.y as i32 + dy) as u16);
        self.snake.body.insert(0, new_head);

        let tail = self.snake.body.pop().unwrap();
        let clear_x = tail.x;
        let clear_y = tail.y;
        self.stdout.queue(MoveTo(clear_x, clear_y))?;
        self.stdout.write(RST.as_bytes())?;
        self.stdout.write(tail.content.as_bytes())?;

        Ok(())
    }

    fn render_snake(&mut self) -> Result<()> {
        let (hx, hy) = (self.snake.body[0].x, self.snake.body[0].y);
        for c in &mut self.snake.body {
            self.stdout.queue(MoveTo(c.x, c.y))?;
            self.stdout.write(c.color.as_bytes())?;
            self.stdout.write(c.content.as_bytes())?;
            self.stdout.write(RST.as_bytes())?;
        }
        self.stdout.queue(MoveTo(hx, hy))?;
        Ok(())
    }

    fn handle_collision(&mut self) -> Result<()> {
        let head = &self.snake.body[0];
        let tail = &self.snake.body.last().unwrap();
        if head.x == 0 || head.x == self.term_size.w || head.y == 0 || head.y == self.term_size.h {
            self.lost = true;
            return Err(Error::other("LOST"));
        }
        let iter = self.snake.body.iter();

        if iter.skip(1).find(|&b| b.x == head.x && b.y == head.y).is_some() {
            self.lost = true;
            return Err(Error::other("LOST"));
        };
        if head.x == self.fruit.x && head.y == self.fruit.y {
            self.fruit = Cell::fruit(self.term_size.w, self.term_size.h);
            let (dx, dy) = self.snake.direction.get_delta();
            self.snake
                .body
                // substracting the delta because we want to expand it in the opposite direction.
                // Now that I think about it this is dumb bc the direction may have changed...
                .push(Cell::snake((tail.x as i32 - dx) as u16, (tail.y as i32 - dy) as u16))
        }
        Ok(())
    }

    fn render_fruit(&mut self) -> Result<()> {
        self.stdout.queue(MoveTo(self.fruit.x, self.fruit.y))?;
        self.stdout.write(self.fruit.color.as_bytes())?;
        self.stdout.write(self.fruit.content.as_bytes())?;
        self.stdout.write(RST.as_bytes())?;
        Ok(())
    }

    fn render_lose(&mut self) -> Result<()> {
        let msg = "UNLUCKY, YOU HAVE LOST";
        self.stdout.queue(Clear(ClearType::All))?;
        self.stdout.queue(MoveTo(
            self.term_size.w / 2 - (msg.len() / 2) as u16,
            self.term_size.h / 2,
        ))?;
        self.stdout.write(msg.as_bytes())?;
        self.stdout.flush()?;
        Ok(())
    }
}
