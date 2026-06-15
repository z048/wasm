#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
compile_error!("wasm32-unknown-unknown only");

use rand::Rng;
use std::sync::LazyLock;
use std::sync::Mutex;
use wasm_bindgen::prelude::wasm_bindgen;
use z048::Board;
use z048::Dicer;
use z048::Rater;
use z048::Slide;
use z048::Spawn;

#[wasm_bindgen]
pub struct Game {
    rater: Rater,
    board: Board,
    phase: bool,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8]) -> Game {
        console_error_panic_hook::set_once();
        let rater = Rater::from(bytes);
        let board = Board::from(DICER.lock().expect("lock global dicer").random::<u64>());
        let phase = false;
        Game { rater, board, phase }
    }

    pub fn board(&self) -> Vec<u8> {
        <[[u8; 4]; 4]>::from(self.board).into_iter().flatten().collect()
    }

    pub fn phase(&self) -> bool {
        self.phase
    }

    pub fn escore(&self) -> f64 {
        self.board.escore()
    }

    pub fn score(&self) -> f64 {
        self.board.score()
    }

    pub fn end(&self) -> bool {
        self.board.end()
    }

    pub fn order(&self, depth: u8, tau: f64) -> Option<u8> {
        if self.end() || self.phase {
            None
        } else {
            let s = self.rater.sample_slide(self.board, depth, tau, &mut DICER.lock().expect("lock global dicer"));
            Some(s as u8)
        }
    }

    pub fn chaos(&self, depth: u8, tau: f64) -> Option<Vec<u8>> {
        if self.phase {
            let s = self.rater.sample_spawn(self.board, depth, tau, &mut DICER.lock().expect("lock global dicer"));
            let ((x, y), rank) = s.cm();
            Some(vec![x as u8, y as u8, rank])
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        self.board = Board::from(DICER.lock().expect("lock global dicer").random::<u64>());
        self.phase = false;
    }

    pub fn slide(&mut self, dir: u8) {
        if dir < 4 {
            let s = Slide::from(dir as u16);
            if !self.phase && self.board.is_legal_slide(s) {
                self.board = self.board.slide(s);
                self.phase = true;
            }
        }
    }
    pub fn spawn(&mut self, x: u8, y: u8, rank: u8) {
        if x < 4 && y < 4 && rank >= 1 && rank <= 2 {
            let s = Spawn::<4, 2>::from(((x as usize, y as usize), rank));
            if self.phase && self.board.is_legal_spawn(s) {
                self.board = self.board.spawn(s);
                self.phase = false;
            }
        }
    }
}

static DICER: LazyLock<Mutex<Dicer>> = LazyLock::new(|| Mutex::new(Dicer::from(rand::random::<u64>())));
