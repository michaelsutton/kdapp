use borsh::{BorshDeserialize, BorshSerialize};
use kdapp::{
    episode::{Episode, EpisodeError, PayloadMetadata},
    pki::PubKey,
};
use log::info;
use std::collections::VecDeque;

#[derive(Debug, BorshDeserialize, BorshSerialize)]
pub enum TTTError {
    OutOfBounds,
    Occupied,
    NotPlayersTurn,
    GameOver,
    NoNewPlayers,
    Unauthorized,
}

impl std::fmt::Display for TTTError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TTTError::OutOfBounds => write!(f, "Move is out of bounds."),
            TTTError::Occupied => write!(f, "Cell is already occupied."),
            TTTError::NotPlayersTurn => write!(f, "It's not this player's turn."),
            TTTError::GameOver => write!(f, "The game is already over."),
            TTTError::NoNewPlayers => write!(f, "Tic-tac-toe does not allow addition of new players."),
            TTTError::Unauthorized => write!(f, "Unauthorized participant."),
        }
    }
}

impl std::error::Error for TTTError {}

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize)]
pub struct TTTMove {
    pub row: usize,
    pub col: usize,
}

#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize)]
pub struct TTTRollback {
    pub mv: TTTMove,
    pub removed_mv: Option<TTTMove>,
    pub prev_timestamp: u64,
}

impl TTTRollback {
    pub fn new(mv: TTTMove, removed_mv: Option<TTTMove>, prev_timestamp: u64) -> Self {
        Self { mv, removed_mv, prev_timestamp }
    }
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct TTTState {
    pub board: [[Option<PubKey>; 3]; 3],
    pub first_player: PubKey,
    pub status: TTTGameStatus,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub enum TTTGameStatus {
    InProgress(PubKey),
    Winner(PubKey),
    Draw,
}

impl TTTState {
    pub fn print(&self) {
        Self::print_board(&self.board, self.first_player);
        match self.status {
            TTTGameStatus::InProgress(_pk) => {}
            TTTGameStatus::Winner(pk) => println!("winner: {} [{}]", if pk == self.first_player { "X" } else { "O" }, pk),
            TTTGameStatus::Draw => println!("---- Draw ----"),
        }
    }

    fn print_board(board: &[[Option<PubKey>; 3]; 3], p1: PubKey) {
        // Iterate over each row with its index (0, 1, or 2)
        for (row_index, row) in board.iter().enumerate() {
            // Iterate over each cell in the row with its index
            for (col_index, cell) in row.iter().enumerate() {
                // Use a match statement to determine which character to print
                let symbol = match cell {
                    Some(p) if *p == p1 => "X",
                    Some(_) => "O",
                    None => " ",
                };

                // Print the symbol with padding for nice spacing
                print!(" {} ", symbol);

                // Print a vertical separator between cells, but not after the last one
                if col_index < 2 {
                    print!("|");
                }
            }

            // After printing all cells in a row, move to the next line
            println!();

            // Print a horizontal separator between rows, but not after the last one
            if row_index < 2 {
                println!("---+---+---");
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TicTacToe {
    pub(crate) board: [[Option<PubKey>; 3]; 3],
    pub(crate) players: Vec<PubKey>,
    current_index: usize,
    timestamp: u64,
    move_history: VecDeque<(usize, usize)>,
}

impl Episode for TicTacToe {
    type Command = TTTMove;
    type CommandRollback = TTTRollback;
    type CommandError = TTTError;

    fn initialize(participants: Vec<PubKey>, metadata: &PayloadMetadata) -> Self {
        info!("[TicTacToe] initialize: {:?}", participants);
        Self {
            board: [[None; 3]; 3],
            players: participants,
            current_index: 0,
            timestamp: metadata.accepting_time,
            move_history: VecDeque::new(),
        }
    }

    fn execute(
        &mut self,
        cmd: &Self::Command,
        authorization: Option<PubKey>,
        metadata: &PayloadMetadata,
    ) -> Result<Self::CommandRollback, EpisodeError<Self::CommandError>> {
        let Some(player) = authorization else {
            return Err(EpisodeError::Unauthorized);
        };
        if player != self.players[self.current_index] {
            return Err(EpisodeError::InvalidCommand(TTTError::NotPlayersTurn));
        }
        if cmd.row >= 3 || cmd.col >= 3 {
            return Err(EpisodeError::InvalidCommand(TTTError::OutOfBounds));
        }

        if self.board[cmd.row][cmd.col].is_some() {
            return Err(EpisodeError::InvalidCommand(TTTError::Occupied));
        }

        info!("[TicTacToe] execute: {:?}, {:?}", player, cmd);

        let mut removed_mv = None;

        // Enforce maximum 6 symbols
        if self.move_history.len() == 6 {
            if let Some((old_row, old_col)) = self.move_history.pop_front() {
                self.board[old_row][old_col] = None;
                removed_mv = Some(TTTMove { row: old_row, col: old_col });
            }
        }

        self.board[cmd.row][cmd.col] = Some(player);
        self.move_history.push_back((cmd.row, cmd.col));

        let old_timestamp = self.timestamp;
        self.timestamp = metadata.accepting_time;

        self.current_index = (self.current_index + 1) % self.players.len();

        Ok(TTTRollback::new(*cmd, removed_mv, old_timestamp))
    }

    fn rollback(&mut self, rollback: TTTRollback) -> bool {
        if self.board[rollback.mv.row][rollback.mv.col].is_none() {
            return false;
        }
        self.timestamp = rollback.prev_timestamp;
        self.board[rollback.mv.row][rollback.mv.col] = None;
        self.current_index = (self.current_index + 1) % self.players.len();
        self.move_history.pop_back();
        // Restore removed cell
        if let Some(removed_mv) = rollback.removed_mv {
            // 6 moves back is always current player
            self.board[removed_mv.row][removed_mv.col] = Some(self.players[self.current_index]);
            self.move_history.push_front((removed_mv.row, removed_mv.col));
        }
        true
    }
}

impl TicTacToe {
    pub fn poll(&self) -> TTTState {
        TTTState {
            board: self.board,
            first_player: self.players[0],
            status: if let Some(winner) = self.check_winner() {
                TTTGameStatus::Winner(winner)
            } else if self.is_draw() {
                TTTGameStatus::Draw
            } else {
                TTTGameStatus::InProgress(self.players[self.current_index])
            },
        }
    }

    fn check_winner(&self) -> Option<PubKey> {
        let b = &self.board;
        let lines = [
            [(0, 0), (0, 1), (0, 2)],
            [(1, 0), (1, 1), (1, 2)],
            [(2, 0), (2, 1), (2, 2)],
            [(0, 0), (1, 0), (2, 0)],
            [(0, 1), (1, 1), (2, 1)],
            [(0, 2), (1, 2), (2, 2)],
            [(0, 0), (1, 1), (2, 2)],
            [(0, 2), (1, 1), (2, 0)],
        ];

        for line in lines.iter() {
            let [(r1, c1), (r2, c2), (r3, c3)] = line;
            if let (Some(p1), Some(p2), Some(p3)) = (b[*r1][*c1], b[*r2][*c2], b[*r3][*c3]) {
                if p1 == p2 && p2 == p3 {
                    return Some(p1);
                }
            }
        }
        None
    }

    fn is_draw(&self) -> bool {
        self.board.iter().all(|row| row.iter().all(|c| c.is_some()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kdapp::{
        engine::{self, EngineMsg as Msg, EpisodeMessage},
        pki::{generate_keypair, sign_message, to_message},
    };

    #[test]
    fn test_ttt_rollback() {
        let ((_s1, p1), (_s2, p2)) = (generate_keypair(), generate_keypair());
        let metadata = PayloadMetadata { accepting_hash: 0u64.into(), accepting_daa: 0, accepting_time: 0, tx_id: 1u64.into() };
        let mut game = TicTacToe::initialize(vec![p1, p2], &metadata);
        let rollback = game.execute(&TTTMove { row: 0, col: 0 }, Some(p1), &metadata).unwrap();
        game.rollback(rollback);
        let _rollback = game.execute(&TTTMove { row: 0, col: 0 }, Some(p1), &metadata).unwrap();
        let _rollback = game.execute(&TTTMove { row: 1, col: 0 }, Some(p2), &metadata).unwrap();
        let _rollback = game.execute(&TTTMove { row: 1, col: 1 }, Some(p1), &metadata).unwrap();
        let _rollback = game.execute(&TTTMove { row: 2, col: 0 }, Some(p2), &metadata).unwrap();
        let _rollback = game.execute(&TTTMove { row: 0, col: 2 }, Some(p1), &metadata).unwrap();
        let _rollback = game.execute(&TTTMove { row: 0, col: 1 }, Some(p2), &metadata).unwrap();

        // Test a 7th move
        assert_eq!(game.move_history.len(), 6);
        let snapshot = game.clone();
        let rollback = game.execute(&TTTMove { row: 2, col: 2 }, Some(p1), &metadata).unwrap();
        assert_eq!(game.move_history.len(), 6);
        assert!(game.rollback(rollback));
        assert_eq!(snapshot, game);
    }

    #[tokio::test]
    async fn test_ttt_engine_rollback() {
        let ((s1, p1), (_s2, p2)) = (generate_keypair(), generate_keypair());
        let episode_id = 11;
        let new_episode = EpisodeMessage::<TicTacToe>::NewEpisode { episode_id, participants: vec![p1, p2] };

        let (sender, receiver) = std::sync::mpsc::channel();
        let mut engine = engine::Engine::<TicTacToe>::new(receiver);
        let engine_task = tokio::task::spawn_blocking(move || {
            engine.start(vec![]);
        });

        let payload = borsh::to_vec(&new_episode).unwrap();
        sender
            .send(Msg::BlkAccepted {
                accepting_hash: 1u64.into(),
                accepting_daa: 0,
                accepting_time: 0,
                associated_txs: vec![(2u64.into(), payload)],
            })
            .unwrap();

        let cmd = TTTMove { row: 0, col: 0 };
        let msg = to_message(&cmd);
        let sig = sign_message(&s1, &msg);
        let step = EpisodeMessage::<TicTacToe>::SignedCommand { episode_id, cmd, pubkey: p1, sig };

        let payload = borsh::to_vec(&step).unwrap();
        sender
            .send(Msg::BlkAccepted {
                accepting_hash: 3u64.into(),
                accepting_daa: 1,
                accepting_time: 1,
                associated_txs: vec![(4u64.into(), payload)],
            })
            .unwrap();

        sender.send(Msg::BlkReverted { accepting_hash: 3u64.into() }).unwrap();

        let payload = borsh::to_vec(&step).unwrap();
        sender
            .send(Msg::BlkAccepted {
                accepting_hash: 5u64.into(),
                accepting_daa: 2,
                accepting_time: 2,
                associated_txs: vec![(4u64.into(), payload)],
            })
            .unwrap();

        sender.send(Msg::Exit).unwrap();
        engine_task.await.unwrap();
    }
}
