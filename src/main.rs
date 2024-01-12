use crossterm::{
    cursor::{Hide, MoveTo},
    event::{poll, read, Event, KeyCode, KeyModifiers},
    terminal::{enable_raw_mode, size, Clear, ClearType},
    QueueableCommand,
};
use rand::Rng;
use std::{
    env::args,
    fmt::Display,
    io::{stdout, Error, Result, Stdout, Write},
    process::exit,
    thread,
    time::{Duration, Instant},
};
use term_color::{BRIGHT_RED_BG, GREEN_BG};

mod term_color;
use crate::term_color::{RST, WHITE_BG};

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
        let mut y: u16 = (rand::thread_rng().gen::<f32>() * (max_y) as f32) as u16;
        if y < 2 {
            y = 2
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

    fn snake_head(x: u16, y: u16) -> Cell<'static> {
        Cell {
            x,
            y,
            content: " ",
            color: GREEN_BG,
        }
    }
}

struct Snake<'a> {
    direction: Direction,
    body: Vec<Cell<'a>>,
}

impl Snake<'_> {
    pub fn new(size: u16) -> Snake<'static> {
        let mut body: Vec<Cell> = Vec::new();
        for i in (0..size).rev() {
            println!("{}", i);
            body.push(Cell::snake(i, 2))
        }
        body[0].color = GREEN_BG;
        Snake {
            direction: Direction::Right,
            body,
        }
    }
}

enum Mode {
    Regular,
    Trim,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Mode::Regular => "Regular",
            Mode::Trim => "Trim",
        };
        write!(f, "{}", s)
    }
}

fn main() -> Result<()> {
    println!("Snakeli - v1");
    println!();
    let mut args = args().skip(1);
    let mut length = 2;
    let mut width = 50u16;
    let mut height = 23u16;
    let mut mode = Mode::Regular;
    let mut vim_mode = false;
    while let Some(next) = args.next() {
        match next.as_str() {
            "--help" => {
                print_usage();
                exit(0);
            }
            "-vim" => {
                vim_mode = true;
            }
            "-w" => {
                width = match args.next().expect("Width to be provided").parse::<u16>() {
                    Ok(w) => w,
                    Err(err) => {
                        eprintln!("ERROR - Cannot parse provided width as u16: {:?}", err);
                        exit(1);
                    }
                }
            }
            "-h" => {
                height = match args.next().expect("Height to be provided").parse::<u16>() {
                    Ok(h) => h,
                    Err(err) => {
                        eprintln!("ERROR - Cannot parse provided height as u16: {:?}", err);
                        exit(1);
                    }
                }
            }
            "-l" => {
                length = match args.next().expect("Length to be provided").parse::<u16>() {
                    Ok(l) => l,
                    Err(err) => {
                        eprintln!("ERROR - Cannot parse provided length as u16: {:?}", err);
                        exit(1);
                    }
                }
            }
            "-m" => {
                mode = match args.next().expect("Mode to be provided").as_str() {
                    "TRIM" | "T" => Mode::Trim,
                    "REGULAR" | "R" => Mode::Regular,
                    _ => {
                        eprintln!("Invalid mode");
                        print_usage();
                        exit(1);
                    }
                }
            }
            _ => {
                eprintln!("Unrecognized arg: {}", next);
                print_usage();
                exit(1)
            }
        }
    }
    if length > width - 2 {
        eprintln!("Length({}) cannot be greater than width - 2({}).", length, width - 2);
        print_usage();
        exit(1)
    }
    match Game::init(width, height, length, mode, vim_mode)?.run() {
        Ok(_) => exit(0),
        Err(err) => {
            eprintln!("ERROR: {:?}", err);
            exit(1)
        }
    }
}

fn print_usage() {
    println!("snakeli [-w 50] [-h 30] [-l 5] [-m TRIM]");
    println!("");
    println!("    --help  print this help");
    println!("      -vim  allow only h(left) j(down) k(up) l(right) keys for movement");
    println!("        -w  width of the board");
    println!("        -h  height of the board");
    println!("        -l  initial length. It has to be less than w-2 (48 by default)");
    println!("        -m  game mode. REGULAR by default:");
    println!("              - TRIM: Snake eats itself");
    println!("              - REGULAR: Snake eats itself");
    println!("");
    println!("Controls:");
    println!("    - `<Control>c`: quit");
    println!("    - `<Space>`: pause");
    println!("    - `n`: increase speed");
    println!("    - `m`: decrease speed");
    println!("    - `<Up> | k | w`: go up (only `k` will work in vim mode)");
    println!("    - `<Down> | j | s`: go down (only `j` will work in vim mode)");
    println!("    - `<Left> | h | a`: go left (only `h` will work in vim mode)");
    println!("    - `<Right> | l | d`: go right (only `l` will work in vim mode)");
}

struct Game<'a> {
    clock: Instant,
    stdout: Stdout,
    term_size: TermSize,
    snake: Snake<'a>,
    fruit: Cell<'a>,
    paused: bool,
    lost: bool,
    exit: bool,
    msg: &'a str,
    length: u16,
    mode: Mode,
    vim_mode: bool,
    ms_per_frame: u64,
}

struct TermSize {
    pub w: u16,
    pub h: u16,
}

impl Game<'_> {
    pub fn init(mut w: u16, mut h: u16, l: u16, mode: Mode, vim_mode: bool) -> Result<Game<'static>> {
        let (term_w, term_h) = size()?;
        w = if term_w < w { term_w } else { w };
        h = if term_h < h + 1 { term_h } else { h + 1 };
        let snake = Snake::new(l);
        snake.body.first().unwrap();
        Ok(Game {
            clock: Instant::now(),
            stdout: stdout(),
            term_size: TermSize { w, h },
            snake: Snake::new(l),
            fruit: Cell::fruit(w, h),
            paused: false,
            lost: false,
            exit: false,
            msg: "",
            length: l,
            mode,
            vim_mode,
            ms_per_frame: 60,
        })
    }

    pub fn run(mut self) -> Result<()> {
        enable_raw_mode()?;
        self.stdout.queue(Clear(ClearType::All))?;
        self.stdout.queue(Hide)?;
        self.stdout.flush()?;
        let mut frames: usize = 0;
        let _frames_start = Instant::now();
        self.render_border()?;

        loop {
            self.clock = Instant::now();
            frames = frames + 1;
            // process input
            self.handle_event()?;
            if self.exit {
                break;
            } else if self.paused {
                self.msg = "PAUSED";
            } else if self.lost {
                self.msg = "YOU HAVE LOST :( press r to restart"
            } else {
                // update game state
                self.update_snake()?;
                match self.handle_collision() {
                    Err(err) if err.to_string() == "LOST" => {
                        self.msg = "YOU HAVE LOST :( press r to restart";
                    }
                    Err(err) => {
                        eprintln!("Error handling collision: {:?}", err);
                    }
                    _ => {}
                }
            }
            // render
            self.render()?;
            let diff = Duration::from_millis(self.ms_per_frame) - Instant::now().duration_since(self.clock);
            if diff.as_millis() > 0 {
                thread::sleep(diff);
            }
        }
        Ok(())
    }

    fn render(&mut self) -> Result<()> {
        self.stdout.queue(Clear(ClearType::All))?;
        self.render_status()?;
        self.render_border()?;
        self.render_snake()?;
        self.render_fruit()?;
        self.stdout.flush()?;
        Ok(())
    }

    fn render_border(&mut self) -> Result<()> {
        self.stdout.queue(MoveTo(0, 1))?;
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
        if poll(Duration::from_millis((self.ms_per_frame as f64 * 0.5) as u64))? {
            match read()? {
                Event::Key(event) => match event.code {
                    KeyCode::Char('c') => {
                        if event.modifiers.contains(KeyModifiers::CONTROL) {
                            self.exit = true;
                        }
                    }
                    KeyCode::Char('r') => {
                        self.clock = Instant::now();
                        self.snake = Snake::new(self.length);
                        self.fruit = Cell::fruit(self.term_size.w, self.term_size.h);
                        self.paused = false;
                        self.lost = false;
                        self.exit = false;
                        self.msg = "";
                        self.stdout.queue(Clear(ClearType::All))?;
                        self.render_border()?;
                        self.stdout.flush()?;
                    }
                    KeyCode::Char('n') => {
                        if self.ms_per_frame > 20 {
                            self.ms_per_frame = self.ms_per_frame - 10
                        }
                    }
                    KeyCode::Char('m') => {
                        if self.ms_per_frame < 500 {
                            self.ms_per_frame = self.ms_per_frame + 10
                        }
                    }
                    KeyCode::Up | KeyCode::Char('w') if !self.paused && !self.vim_mode => match self.snake.direction {
                        Direction::Down => {}
                        _ => self.snake.direction = Direction::Up,
                    },
                    KeyCode::Down | KeyCode::Char('s') if !self.paused && !self.vim_mode => {
                        match self.snake.direction {
                            Direction::Up => {}
                            _ => self.snake.direction = Direction::Down,
                        }
                    }
                    KeyCode::Left | KeyCode::Char('a') if !self.paused && !self.vim_mode => {
                        match self.snake.direction {
                            Direction::Right => {}
                            _ => self.snake.direction = Direction::Left,
                        }
                    }
                    KeyCode::Right | KeyCode::Char('d') if !self.paused && !self.vim_mode => {
                        match self.snake.direction {
                            Direction::Left => {}
                            _ => self.snake.direction = Direction::Right,
                        }
                    }
                    KeyCode::Char('h') if !self.paused => match self.snake.direction {
                        Direction::Right => {}
                        _ => self.snake.direction = Direction::Left,
                    },
                    KeyCode::Char('j') if !self.paused => match self.snake.direction {
                        Direction::Up => {}
                        _ => self.snake.direction = Direction::Down,
                    },
                    KeyCode::Char('k') if !self.paused => match self.snake.direction {
                        Direction::Down => {}
                        _ => self.snake.direction = Direction::Up,
                    },
                    KeyCode::Char('l') if !self.paused => match self.snake.direction {
                        Direction::Left => {}
                        _ => self.snake.direction = Direction::Right,
                    },
                    KeyCode::Char(' ') if !self.lost => {
                        self.paused = !self.paused;
                        self.msg = if self.paused { "PAUSED" } else { "" }
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
        let head = &mut self.snake.body[0];
        head.color = WHITE_BG;
        let (dx, dy) = self.snake.direction.get_delta();
        let new_head = Cell::snake_head((head.x as i32 + dx) as u16, (head.y as i32 + dy) as u16);
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
        for c in &mut self.snake.body.iter().rev() {
            self.stdout.queue(MoveTo(c.x, c.y))?;
            self.stdout.write(c.color.as_bytes())?;
            self.stdout.write(c.content.as_bytes())?;
            self.stdout.write(RST.as_bytes())?;
        }
        Ok(())
    }

    fn handle_collision(&mut self) -> Result<()> {
        let head = &self.snake.body[0];
        let tail = &self.snake.body.last().unwrap();
        if head.x == 0 || head.x == self.term_size.w - 1 || head.y == 1 || head.y == self.term_size.h {
            self.lost = true;
            return Err(Error::other("LOST"));
        }

        for i in 1..self.snake.body.len() {
            if head.x == self.snake.body[i].x && head.y == self.snake.body[i].y {
                match self.mode {
                    Mode::Regular => {
                        self.lost = true;
                        return Err(Error::other("LOST"));
                    }
                    Mode::Trim => {
                        let body = &mut self.snake.body;
                        body.truncate(i);
                        return Ok(());
                    }
                }
            }
        }

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

    fn render_status(&mut self) -> Result<()> {
        self.stdout.queue(MoveTo(0, 0))?;
        self.stdout.write(" ".repeat(self.term_size.w as usize).as_bytes())?;
        self.stdout.queue(MoveTo(0, 0))?;
        self.stdout
            .write(format!("Score: {}   ", self.snake.body.len()).as_bytes())?;
        self.stdout.write(self.msg.as_bytes())?;
        Ok(())
    }
}
