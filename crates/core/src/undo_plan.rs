use shakmaty::Chess;
use shakmaty::Piece;
use shakmaty::Position;
use shakmaty::Square;

use crate::UndoStep;

/// Physical steps needed to revert one move on the board.
///
/// Computed by diffing the position before and after the move, which
/// uniformly covers normal moves, captures, en passant, castling (two
/// `MoveBack` steps) and promotion (a `MoveBack`; the promoted role is a
/// logical detail the marker cannot show).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoPlan {
    /// Ordered physical steps: `MoveBack` first (they free the
    /// destination squares), `Restore` afterwards.
    pub steps: Vec<UndoStep>,
}

impl UndoPlan {
    /// Computes the steps that turn `after` back into `before`.
    ///
    /// Algorithm: collect pieces that left their square (`removed`) and
    /// pieces that arrived (`arrived`); match each arrival to a removed
    /// piece of the same color (exact piece match first, promotion is a
    /// pawn→role mismatch on the same side); unmatched removals are
    /// captured pieces that must be placed back.
    pub fn between(before: &Chess, after: &Chess) -> Self {
        let mut removed: Vec<(Square, Piece)> = Vec::new();
        let mut arrived: Vec<(Square, Piece)> = Vec::new();
        for square in Square::ALL {
            let was = before.board().piece_at(square);
            let is = after.board().piece_at(square);
            if was == is {
                continue;
            }
            if let Some(piece) = was {
                removed.push((square, piece));
            }
            if let Some(piece) = is {
                arrived.push((square, piece));
            }
        }

        let mut steps = Vec::new();
        let mut consumed: Vec<usize> = Vec::new();
        for (destination, piece) in &arrived {
            // Exact match first, then the promotion fallback: the same
            // color's pawn vacated a square.
            let source = removed
                .iter()
                .enumerate()
                .find(|(index, (_sq, removed_piece))| !consumed.contains(index) && removed_piece == piece)
                .or_else(|| {
                    removed
                        .iter()
                        .enumerate()
                        .find(|(index, (_sq, removed_piece))| !consumed.contains(index) && removed_piece.color == piece.color)
                });
            if let Some((index, (from, _))) = source {
                consumed.push(index);
                steps.push(UndoStep::MoveBack { from: *from, to: *destination });
            }
        }
        for (index, (square, piece)) in removed.iter().enumerate() {
            if !consumed.contains(&index) {
                steps.push(UndoStep::Restore {
                    square: *square,
                    piece: *piece,
                });
            }
        }
        Self { steps }
    }

    /// Whether the plan contains no steps.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use shakmaty::CastlingMode;
    use shakmaty::Chess;
    use shakmaty::Color;
    use shakmaty::Piece;
    use shakmaty::Position;
    use shakmaty::Role;
    use shakmaty::Square;
    use shakmaty::fen::Fen;
    use shakmaty::uci::UciMove;

    use super::UndoPlan;
    use crate::UndoStep;

    fn play_uci(pos: &Chess, uci: &str) -> Chess {
        let mv = uci.parse::<UciMove>().expect("uci").to_move(pos).expect("legal");
        pos.clone().play(mv).expect("play")
    }

    #[test]
    fn normal_move_yields_single_move_back() {
        let before = Chess::default();
        let after = play_uci(&before, "e2e4");
        let plan = UndoPlan::between(&before, &after);
        assert_eq!(
            plan.steps,
            vec![UndoStep::MoveBack {
                from: Square::E2,
                to: Square::E4
            }]
        );
    }

    #[test]
    fn capture_restores_captured_piece() {
        // Position after 1.e4 d5 - white to move plays exd5.
        let before = Fen::from_ascii(b"rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2")
            .expect("fen")
            .into_position::<Chess>(CastlingMode::Standard)
            .expect("pos");
        let after = play_uci(&before, "e4d5");
        let plan = UndoPlan::between(&before, &after);
        assert_eq!(
            plan.steps,
            vec![
                UndoStep::MoveBack {
                    from: Square::E4,
                    to: Square::D5
                },
                UndoStep::Restore {
                    square: Square::D5,
                    piece: Piece {
                        color: Color::Black,
                        role: Role::Pawn
                    }
                }
            ]
        );
    }

    #[test]
    fn en_passant_restores_pawn_on_adjacent_square() {
        // Position after 1.e4 a6 2.e5 d5 - white plays exd6 e.p.
        // (d7 empty because the pawn double-moved; b7 kept occupied.)
        let before = Fen::from_ascii(b"rnbqkbnr/p1p1pppp/p7/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3")
            .expect("fen")
            .into_position::<Chess>(CastlingMode::Standard)
            .expect("pos");
        let after = play_uci(&before, "e5d6");
        let plan = UndoPlan::between(&before, &after);
        assert_eq!(
            plan.steps,
            vec![
                UndoStep::MoveBack {
                    from: Square::E5,
                    to: Square::D6
                },
                UndoStep::Restore {
                    square: Square::D5,
                    piece: Piece {
                        color: Color::Black,
                        role: Role::Pawn
                    }
                }
            ]
        );
    }

    #[test]
    fn castling_yields_two_move_backs() {
        let before = Fen::from_ascii(b"r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1")
            .expect("fen")
            .into_position::<Chess>(CastlingMode::Standard)
            .expect("pos");
        let after = play_uci(&before, "e1g1");
        let plan = UndoPlan::between(&before, &after);
        assert_eq!(plan.steps.len(), 2);
        assert!(plan.steps.contains(&UndoStep::MoveBack {
            from: Square::E1,
            to: Square::G1
        }));
        assert!(plan.steps.contains(&UndoStep::MoveBack {
            from: Square::H1,
            to: Square::F1
        }));
    }

    #[test]
    fn promotion_moves_back_single_step() {
        let before = Fen::from_ascii(b"8/1P6/8/8/8/8/k6K/8 w - - 0 1")
            .expect("fen")
            .into_position::<Chess>(CastlingMode::Standard)
            .expect("pos");
        let after = play_uci(&before, "b7b8q");
        let plan = UndoPlan::between(&before, &after);
        assert_eq!(
            plan.steps,
            vec![UndoStep::MoveBack {
                from: Square::B7,
                to: Square::B8
            }]
        );
    }
}
