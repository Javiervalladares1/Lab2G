use minifb::{Key, Scale, ScaleMode, Window, WindowOptions};
use rand::Rng;
use std::thread::sleep;
use std::time::{Duration, Instant};

const GRID_W: usize = 120;
const GRID_H: usize = 120;

// Si tu versión de minifb no soporta X2, cambia a X1.
const WINDOW_SCALE: Scale = Scale::X2;

// Colores ARGB
const DEAD: u32 = 0xFF000000;
const ALIVE: u32 = 0xFFFFFFFF;

// ==================== Framebuffer ====================

struct Framebuffer {
    w: usize,
    h: usize,
    buf: Vec<u32>, // 0xAARRGGBB
}

impl Framebuffer {
    fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            buf: vec![DEAD; w * h],
        }
    }

    #[inline]
    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }

    /// Dibuja UN píxel.
    fn point(&mut self, x: usize, y: usize, color: u32) {
        if x < self.w && y < self.h {
            let i = self.idx(x, y);
            self.buf[i] = color;
        }
    }

    /// Lee el color actual de un píxel.
    fn get_color(&self, x: usize, y: usize) -> u32 {
        if x < self.w && y < self.h {
            self.buf[self.idx(x, y)]
        } else {
            DEAD
        }
    }

    fn clear(&mut self, color: u32) {
        self.buf.fill(color);
    }
}

// ==================== Game of Life Core ====================

struct GameOfLife {
    w: usize,
    h: usize,
    curr: Vec<u8>,
    next: Vec<u8>,
    paused: bool,
    step_once: bool,
    delay_ms: u64,
}

impl GameOfLife {
    fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            curr: vec![0; w * h],
            next: vec![0; w * h],
            paused: false,
            step_once: false,
            delay_ms: 100,
        }
    }

    #[inline]
    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }

    #[inline]
    fn set_alive(&mut self, x: usize, y: usize) {
        if x < self.w && y < self.h {
            let i = self.idx(x, y);
            self.curr[i] = 1;
        }
    }

    #[inline]
    fn set_dead(&mut self, x: usize, y: usize) {
        if x < self.w && y < self.h {
            let i = self.idx(x, y);
            self.curr[i] = 0;
        }
    }

    #[inline]
    fn is_alive(&self, x: usize, y: usize) -> bool {
        self.curr[self.idx(x, y)] == 1
    }

    fn clear(&mut self) {
        self.curr.fill(0);
        self.next.fill(0);
    }

    fn randomize(&mut self, prob_alive: f32) {
        let mut rng = rand::thread_rng();
        for c in &mut self.curr {
            *c = if rng.gen::<f32>() < prob_alive { 1 } else { 0 };
        }
    }

    /// Vecinos con wrap-around (toroidal).
    fn live_neighbors(&self, x: usize, y: usize) -> u8 {
        let mut count = 0u8;
        for dy in [-1isize, 0, 1] {
            for dx in [-1isize, 0, 1] {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = ((x as isize + dx).rem_euclid(self.w as isize)) as usize;
                let ny = ((y as isize + dy).rem_euclid(self.h as isize)) as usize;
                count += self.curr[self.idx(nx, ny)];
            }
        }
        count
    }

    /// Aplica las reglas de Conway.
    fn step(&mut self) {
        for y in 0..self.h {
            for x in 0..self.w {
                let alive = self.curr[self.idx(x, y)] == 1;
                let n = self.live_neighbors(x, y);

                let new_state = match (alive, n) {
                    (true, n) if n < 2 => 0,           // underpopulation
                    (true, 2) | (true, 3) => 1,        // survival
                    (true, n) if n > 3 => 0,           // overpopulation
                    (false, 3) => 1,                   // reproduction
                    _ => 0,
                };

                let i = self.idx(x, y);
                self.next[i] = new_state;
            }
        }
        std::mem::swap(&mut self.curr, &mut self.next);
    }

    /// Dibuja el estado actual en el framebuffer.
    fn render_to(&self, fb: &mut Framebuffer) {
        for y in 0..self.h {
            for x in 0..self.w {
                let color = if self.is_alive(x, y) { ALIVE } else { DEAD };
                fb.point(x, y, color);
            }
        }
    }

    // ====== Utilidad genérica para patrones ======

    /// Carga un patrón desde ASCII. Cada línea es una fila. `alive` define el char “vivo”.
    fn spawn_pattern_ascii(&mut self, ox: usize, oy: usize, pattern: &str, alive: char) {
        for (dy, line) in pattern.lines().enumerate() {
            for (dx, ch) in line.chars().enumerate() {
                if ch == alive {
                    let x = (ox + dx) % self.w;
                    let y = (oy + dy) % self.h;
                    let i = self.idx(x, y);
                    self.curr[i] = 1;
                }
            }
        }
    }

    // ====== Patrones clásicos ======

    // Still lifes
    fn spawn_block(&mut self, x: usize, y: usize) {
        let coords = [(0, 0), (1, 0), (0, 1), (1, 1)];
        self.spawn(&coords, x, y);
    }

    fn spawn_beehive(&mut self, x: usize, y: usize) {
        let coords = [(1, 0), (2, 0), (0, 1), (3, 1), (1, 2), (2, 2)];
        self.spawn(&coords, x, y);
    }

    fn spawn_loaf(&mut self, x: usize, y: usize) {
        let coords = [
            (1, 0),
            (2, 0),
            (0, 1),
            (3, 1),
            (1, 2),
            (3, 2),
            (2, 3),
        ];
        self.spawn(&coords, x, y);
    }

    fn spawn_boat(&mut self, x: usize, y: usize) {
        let coords = [(0, 0), (1, 0), (0, 1), (2, 1), (1, 2)];
        self.spawn(&coords, x, y);
    }

    fn spawn_tub(&mut self, x: usize, y: usize) {
        let coords = [(1, 0), (0, 1), (2, 1), (1, 2)];
        self.spawn(&coords, x, y);
    }

    // Oscillators
    fn spawn_blinker(&mut self, x: usize, y: usize) {
        let coords = [(0, 0), (1, 0), (2, 0)];
        self.spawn(&coords, x, y);
    }

    fn spawn_toad(&mut self, x: usize, y: usize) {
        let coords = [(1, 0), (2, 0), (3, 0), (0, 1), (1, 1), (2, 1)];
        self.spawn(&coords, x, y);
    }

    fn spawn_beacon(&mut self, x: usize, y: usize) {
        let coords = [(0, 0), (1, 0), (0, 1), (1, 1), (2, 2), (3, 2), (2, 3), (3, 3)];
        self.spawn(&coords, x, y);
    }

    fn spawn_pulsar(&mut self, x: usize, y: usize) {
        let rel = [
            (2, 0),
            (3, 0),
            (4, 0),
            (8, 0),
            (9, 0),
            (10, 0),
            (0, 2),
            (5, 2),
            (7, 2),
            (12, 2),
            (0, 3),
            (5, 3),
            (7, 3),
            (12, 3),
            (0, 4),
            (5, 4),
            (7, 4),
            (12, 4),
            (2, 5),
            (3, 5),
            (4, 5),
            (8, 5),
            (9, 5),
            (10, 5),
            (2, 7),
            (3, 7),
            (4, 7),
            (8, 7),
            (9, 7),
            (10, 7),
            (0, 8),
            (5, 8),
            (7, 8),
            (12, 8),
            (0, 9),
            (5, 9),
            (7, 9),
            (12, 9),
            (0, 10),
            (5, 10),
            (7, 10),
            (12, 10),
            (2, 12),
            (3, 12),
            (4, 12),
            (8, 12),
            (9, 12),
            (10, 12),
        ];
        self.spawn(&rel, x, y);
    }

    fn spawn_pentadecathlon(&mut self, x: usize, y: usize) {
        let rel = [
            (1, 0),
            (2, 0),
            (1, 1),
            (2, 1),
            (1, 3),
            (2, 3),
            (1, 4),
            (2, 4),
            (1, 5),
            (2, 5),
            (1, 6),
            (2, 6),
            (1, 8),
            (2, 8),
            (1, 9),
            (2, 9),
        ];
        self.spawn(&rel, x, y);
    }

    // Spaceships
    fn spawn_glider(&mut self, x: usize, y: usize) {
        let coords = [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)];
        self.spawn(&coords, x, y);
    }

    fn spawn_lwss(&mut self, x: usize, y: usize) {
        // Lightweight spaceship
        let rel = [
            (1, 0),
            (4, 0),
            (0, 1),
            (0, 2),
            (4, 2),
            (0, 3),
            (1, 3),
            (2, 3),
            (3, 3),
        ];
        self.spawn(&rel, x, y);
    }

    /// Middle-weight spaceship (MWSS)
    fn spawn_mwss(&mut self, x: usize, y: usize) {
        let pat = "\
..####.\n\
.#....#\n\
#.....#\n\
#....#.\n\
.#####.";
        self.spawn_pattern_ascii(x, y, pat, '#');
    }

    /// Heavy-weight spaceship (HWSS)
    fn spawn_hwss(&mut self, x: usize, y: usize) {
        let pat = "\
..#####.\n\
.#.....#\n\
#......#\n\
#.....#.\n\
.######.";
        self.spawn_pattern_ascii(x, y, pat, '#');
    }

    /// R‑pentomino
    fn spawn_r_pentomino(&mut self, x: usize, y: usize) {
        let pat = "\
.##\n\
##.\n\
.#.";
        self.spawn_pattern_ascii(x, y, pat, '#');
    }

    /// Diehard
    fn spawn_diehard(&mut self, x: usize, y: usize) {
        let pat = "\
......#.\n\
##......\n\
.#...###";
        self.spawn_pattern_ascii(x, y, pat, '#');
    }

    /// Acorn
    fn spawn_acorn(&mut self, x: usize, y: usize) {
        let pat = "\
.#.....\n\
...#...\n\
##..###";
        self.spawn_pattern_ascii(x, y, pat, '#');
    }

    // Util para spawnear un patrón relativo a (x,y)
    fn spawn(&mut self, rel: &[(usize, usize)], ox: usize, oy: usize) {
        for (dx, dy) in rel {
            let x = (ox + dx) % self.w;
            let y = (oy + dy) % self.h;
            let i = self.idx(x, y);
            self.curr[i] = 1;
        }
    }
}

// ==================== Main / Engine ====================

fn main() {
    let mut window = Window::new(
        "Conway's Game of Life (Rust) - Space: pausa | N: step | R: random | C: clear | 1/2/3: velocidad | Esc: salir",
        GRID_W,
        GRID_H,
        WindowOptions {
            scale: WINDOW_SCALE,
            scale_mode: ScaleMode::Stretch,
            ..WindowOptions::default()
        },
    )
    .expect("No se pudo crear la ventana");

    let mut fb = Framebuffer::new(GRID_W, GRID_H);
    let mut gol = GameOfLife::new(GRID_W, GRID_H);

    // ------------ Patrón inicial creativo (mezcla) ------------
    // Still lifes
    gol.spawn_block(5, 5);
    gol.spawn_beehive(15, 5);
    gol.spawn_loaf(30, 5);
    gol.spawn_boat(45, 5);
    gol.spawn_tub(60, 5);

    // Oscillators
    gol.spawn_blinker(10, 30);
    gol.spawn_toad(20, 30);
    gol.spawn_beacon(30, 28);
    gol.spawn_pulsar(60, 25);
    gol.spawn_pentadecathlon(90, 15);

    // Spaceships
    gol.spawn_glider(5, 80);
    gol.spawn_lwss(20, 85);
    gol.spawn_mwss(40, 85);
    gol.spawn_hwss(70, 85);

    // Extras que “rompen” la pantalla con el tiempo
    gol.spawn_r_pentomino(100, 60);
    gol.spawn_diehard(5, 100);
    gol.spawn_acorn(80, 90);
    // -----------------------------------------------------------

    let mut last_step = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Input
        if window.is_key_pressed(Key::Space, minifb::KeyRepeat::No) {
            gol.paused = !gol.paused;
        }
        if window.is_key_pressed(Key::N, minifb::KeyRepeat::No) {
            gol.step_once = true;
        }
        if window.is_key_pressed(Key::C, minifb::KeyRepeat::No) {
            gol.clear();
        }
        if window.is_key_pressed(Key::R, minifb::KeyRepeat::No) {
            gol.randomize(0.20);
        }
        if window.is_key_pressed(Key::Key1, minifb::KeyRepeat::No) {
            gol.delay_ms = 200;
        }
        if window.is_key_pressed(Key::Key2, minifb::KeyRepeat::No) {
            gol.delay_ms = 100;
        }
        if window.is_key_pressed(Key::Key3, minifb::KeyRepeat::No) {
            gol.delay_ms = 16;
        }

        // Update (step) con timing simple
        let should_step = if gol.paused {
            gol.step_once
        } else {
            last_step.elapsed() >= Duration::from_millis(gol.delay_ms)
        };

        if should_step {
            gol.step();
            last_step = Instant::now();
            gol.step_once = false;
        }

        // Render
        gol.render_to(&mut fb);

        window
            .update_with_buffer(&fb.buf, fb.w, fb.h)
            .expect("No se pudo actualizar el framebuffer");

        // Evitar usar 100% CPU cuando está en pausa
        if gol.paused && !gol.step_once {
            sleep(Duration::from_millis(10));
        }
    }
}
