//! `host.rs`'s `relay_tests`, in a file of its own so that `host.rs`
//! stays under the repository's per-file size limit.

    use super::{fetched_beside, flagged_pieces, landable, note_beside};

    /// A piece fetched on the copy queue while a later flight was in the
    /// air is fetched again by that flight, and by no flight sent after
    /// the fetch had run; the note goes when its flights have landed.
    #[test]
    fn a_piece_fetched_beside_a_later_flight_is_fetched_again_by_it() {
        let mut noted = std::collections::HashMap::new();
        let (a, p) = (0x1000usize, 7u32);
        // Flight 5 lands; its fetch ran while 6 and 7 had started.
        assert_eq!(note_beside(&mut noted, &[(a, p)], 5, 7), 1);
        assert!(fetched_beside(noted.get(&(a, p)).copied(), 6));
        assert!(fetched_beside(noted.get(&(a, p)).copied(), 7));
        // Flight 8 was sent after the fetch had run: its compare decides.
        assert!(!fetched_beside(noted.get(&(a, p)).copied(), 8));
        // Another piece was never noted.
        assert!(!fetched_beside(noted.get(&(a, p + 1)).copied(), 6));
        // Flight 6 lands and fetches it again, with 7 still in the air and
        // nothing newer: the note stays for 7.
        assert_eq!(note_beside(&mut noted, &[(a, p)], 6, 7), 1);
        assert!(fetched_beside(noted.get(&(a, p)).copied(), 7));
        // Flight 7 lands with nothing behind it: nothing is noted any more.
        assert_eq!(note_beside(&mut noted, &[(a, p)], 7, 7), 0);
        assert!(noted.is_empty());
        // A fetch with nothing later in the air notes nothing.
        assert_eq!(note_beside(&mut noted, &[(a, p)], 9, 9), 0);
        assert!(noted.is_empty());
    }

    /// Two flights and what follows each: nothing of the second lands
    /// before the card has run it, and nothing behind a flight lands
    /// before the flight.
    #[test]
    fn flights_land_in_the_order_they_were_sent() {
        // Flight (fence 5), its write, a signal, flight (fence 6), its write.
        let line = [Some(5), None, None, Some(6), None];
        assert_eq!(landable(&line, 4), 0);
        assert_eq!(landable(&line, 5), 3);
        assert_eq!(landable(&line, 6), 5);
        assert_eq!(landable(&line, u64::MAX), 5);
        // Signals with no flight ahead land at once; behind one they wait.
        assert_eq!(landable(&[None, None], 0), 2);
        assert_eq!(landable(&[None, Some(9), None], 8), 1);
        assert_eq!(landable(&[], 3), 0);
        // After the first three landed, the rest when its turn comes.
        assert_eq!(landable(&line[3..], 5), 0);
        assert_eq!(landable(&line[3..], 6), 2);
    }

    /// A piece flagged under one range of a plan is fetched under every
    /// range that holds a part of it; pieces of other copies are not.
    #[test]
    fn a_piece_flagged_under_one_range_is_fetched_under_all() {
        let (a, b) = (0x1000usize, 0x2000usize);
        // Copy a: piece 3 under two ranges, flagged under the first only;
        // piece 4 not flagged; copy b: piece 3 not flagged, piece 9 flagged.
        let flags = [(a, 3, true), (a, 4, false), (a, 3, false), (b, 3, false), (b, 9, true)];
        let fetch = flagged_pieces(&flags);
        assert!(fetch.contains(&(a, 3)));
        assert!(!fetch.contains(&(a, 4)));
        assert!(!fetch.contains(&(b, 3)));
        assert!(fetch.contains(&(b, 9)));
        assert_eq!(fetch.len(), 2);
    }
