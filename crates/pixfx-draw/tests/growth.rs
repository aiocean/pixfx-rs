use pixfx_draw::growth::{Colonize, Dbm, Tree, colonize, dbm};
use pixfx_draw::path::Path;

fn disc(x: i32, y: i32) -> bool {
    x * x + y * y <= 30 * 30
}

/// Grown from the bottom of a disc toward its top, the walls insulating.
fn bolt(eta: f64, seed: f64) -> Tree {
    dbm(
        &[(0, 29)],
        &[(0, -29)],
        Dbm {
            eta,
            edge: None,
            cells: 300,
            bounds: (-30, -30, 30, 30),
            seed,
            ..Dbm::default()
        },
        disc,
    )
}

#[test]
fn dbm_is_the_same_from_the_same_seed_and_differs_by_seed() {
    assert_eq!(bolt(2.0, 1.0), bolt(2.0, 1.0));
    assert_ne!(bolt(2.0, 1.0), bolt(2.0, 2.0));
}

#[test]
fn dbm_never_grows_through_the_wall() {
    let t = dbm(
        &[(0, 0)],
        &[],
        Dbm {
            eta: 1.0,
            cells: 400,
            bounds: (-30, -30, 30, 30),
            seed: 3.0,
            ..Dbm::default()
        },
        disc,
    );
    assert_eq!(t.nodes.len(), 401);
    assert!(t.nodes.iter().all(|n| disc(n.x as i32, n.y as i32)));
}

#[test]
fn dbm_aimed_reaches_its_target_and_stops() {
    let t = dbm(
        &[(0, -28)],
        &[(10, 28)],
        Dbm {
            eta: 4.0,
            edge: None,
            cells: 2000,
            reach: 1.5,
            bounds: (-30, -30, 30, 30),
            seed: 5.0,
            ..Dbm::default()
        },
        disc,
    );
    let last = t.nodes.last().copied().unwrap_or_else(|| unreachable!());
    let (dx, dy) = (last.x - 10.0, last.y - 28.0);
    let d = libm::sqrt(dx * dx + dy * dy);
    assert!(d <= 1.5, "stopped {d} from the target");
    assert!(
        t.nodes.len() < 400,
        "an aimed bolt goes nearly straight: {} cells",
        t.nodes.len()
    );
}

#[test]
fn dbm_eta_trades_branches_for_a_single_bolt() {
    // a high eta heads for the target with few side branches, so it gets there in fewer cells
    let cells = |eta: f64| {
        dbm(
            &[(0, 29)],
            &[(0, -29)],
            Dbm {
                eta,
                edge: None,
                cells: 3000,
                reach: 1.5,
                bounds: (-30, -30, 30, 30),
                seed: 7.0,
                ..Dbm::default()
            },
            disc,
        )
        .nodes
        .len()
    };
    let (coral, bolt6) = (cells(1.0), cells(6.0));
    assert!(coral > bolt6 * 2, "eta 1 took {coral} cells, eta 6 {bolt6}");
}

#[test]
fn weights_count_each_subtree_and_parents_come_first() {
    let t = bolt(2.0, 9.0);
    let roots: u32 = t
        .nodes
        .iter()
        .filter(|n| n.parent.is_none())
        .map(|n| n.weight)
        .sum();
    assert_eq!(roots as usize, t.nodes.len());
    for (i, n) in t.nodes.iter().enumerate() {
        if let Some(p) = n.parent {
            assert!(p < i);
            let parent = t.nodes.get(p).copied().unwrap_or_else(|| unreachable!());
            assert!(parent.weight > n.weight);
            assert!(parent.born <= n.born);
        }
    }
    assert_eq!(t.heaviest() as usize, t.nodes.len());
}

#[test]
fn radii_taper_smoothly_along_each_branch() {
    let t = bolt(4.0, 2.0);
    let taper = 0.1;
    let r = t.radii(0.5, 2.5, taper);
    for (i, n) in t.nodes.iter().enumerate() {
        let (Some(p), Some(&ri)) = (n.parent, r.get(i)) else {
            continue;
        };
        let rp = r.get(p).copied().unwrap_or(0.0);
        assert!(ri <= rp + 1e-9, "a child is never wider than its parent");
        // the heaviest child (the first, on a tie) continues the line: no step wider than the taper
        let main = t
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, m)| m.parent == Some(p))
            .fold(None, |best: Option<(usize, u32)>, (j, m)| match best {
                Some((_, w)) if w >= m.weight => best,
                _ => Some((j, m.weight)),
            });
        if main.map(|(j, _)| j) == Some(i) && ri > 0.5 + 1e-9 {
            assert!(
                rp - ri <= taper + 1e-9,
                "the line steps by {} at node {i}",
                rp - ri
            );
        }
    }
    let mut covered = 0;
    t.stroke(&r, |dab| {
        assert!((0.0..=1.0).contains(&dab.depth));
        let (nx, ny) = dab.normal;
        let l = nx * nx + ny * ny;
        assert!(
            l == 0.0 || (l - 1.0).abs() < 1e-9,
            "the normal is a unit vector"
        );
        covered += 1;
    });
    assert!(covered >= t.nodes.len());
}

#[test]
fn colonize_fills_its_region_from_a_root_outside_it() {
    let inside = |x: f64, y: f64| (0.0..=40.0).contains(&y) && libm::fabs(x) <= 30.0;
    let t = colonize(
        &[(-20.0, -3.0), (20.0, -3.0)],
        Colonize {
            attractors: 300,
            step: 2.0,
            influence: 16.0,
            kill: 3.0,
            steps: 160,
            bias: (0.0, 0.0),
            jitter: 0.0,
            branch_chance: 1.0,
            max_turn: core::f64::consts::PI,
            fork_angle: (0.0, core::f64::consts::PI),
            bounds: (-30.0, 0.0, 30.0, 40.0),
            seed: 4.0,
        },
        inside,
    );
    // once a vein is inside it never leaves
    for n in &t.nodes {
        let was_in = n
            .parent
            .and_then(|p| t.nodes.get(p))
            .is_some_and(|p| inside(p.x, p.y));
        assert!(!was_in || inside(n.x, n.y));
    }
    let (mut near, mut all) = (0, 0);
    for y in (2..40).step_by(4) {
        for x in (-28..=28).step_by(4) {
            all += 1;
            let (x, y) = (f64::from(x), f64::from(y));
            if t.nodes
                .iter()
                .any(|n| (n.x - x) * (n.x - x) + (n.y - y) * (n.y - y) <= 8.0 * 8.0)
            {
                near += 1;
            }
        }
    }
    assert!(near * 10 >= all * 9, "{near} of {all} probes near a vein");
}

/// A trunk up the y axis that splits at (0, -10) into two branches of `left` and `right` nodes, both leaving at
/// right angles.
fn fork(left: usize, right: usize) -> Tree {
    let mut t = Tree::roots(&[(0.0, 0.0)]);
    t.path(&[(0.0, 0.0), (0.0, -10.0)], 1.0);
    let tip =
        |n: usize, side: f64| -> Vec<(f64, f64)> { vec![(0.0, -10.0), (side * n as f64, -10.0)] };
    t.path(&tip(left, -1.0), 1.0);
    t.path(&tip(right, 1.0), 1.0);
    t
}

/// The angle off upright of the first node of the branch going toward `side`.
fn leaves_at(t: &Tree, side: f64) -> f64 {
    let fork = t
        .nodes
        .iter()
        .position(|n| n.x == 0.0 && n.y == -10.0)
        .unwrap_or(0);
    let first = t
        .nodes
        .iter()
        .find(|n| n.parent == Some(fork) && n.x * side > 0.0)
        .copied()
        .unwrap_or_else(|| unreachable!());
    libm::atan2(libm::fabs(first.x), -(first.y + 10.0)).to_degrees()
}

#[test]
fn murray_forks_an_even_split_symmetrically_and_a_thin_branch_wider() {
    let mut even = fork(20, 20);
    even.murray(1.0, 1.0, 0, |_, _| true);
    let (l, r) = (leaves_at(&even, -1.0), leaves_at(&even, 1.0));
    assert!(
        (l - r).abs() < 1.0,
        "an even split is symmetric: {l} and {r}"
    );
    assert!(
        (30.0..45.0).contains(&l),
        "an even split forks at about 37 degrees: {l}"
    );

    let mut uneven = fork(30, 3);
    uneven.murray(1.0, 1.0, 0, |_, _| true);
    let (main, thin) = (leaves_at(&uneven, -1.0), leaves_at(&uneven, 1.0));
    assert!(
        thin > l && main < l,
        "the thin branch leaves wider ({thin}), the main barely turns ({main})"
    );

    // nodes before keep never move, and a turn out of bounds is not made
    let mut kept = fork(20, 20);
    let before = kept.clone();
    kept.murray(1.0, 1.0, kept.nodes.len(), |_, _| true);
    let at = |t: &Tree| t.nodes.iter().map(|n| (n.x, n.y)).collect::<Vec<_>>();
    assert_eq!(at(&kept), at(&before));
    let mut walled = fork(20, 20);
    walled.murray(1.0, 1.0, 0, |_, y| y > -10.5);
    assert_eq!(at(&walled), at(&before));
}

#[test]
fn growth_laid_along_a_path_goes_round_it_from_its_start() {
    // from (-20, 0) over the top of a ring to (20, 0), not straight across
    let way = Path::begin((-20.0, 0.0)).turn((0.0, 0.0), core::f64::consts::PI);
    let mut t = Tree::default();
    t.path(&way.points(1.0), 1.0);
    for pair in t.nodes.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        assert!(
            (libm::hypot(b.x, b.y) - 20.0).abs() < 0.2,
            "off the ring at ({}, {})",
            b.x,
            b.y
        );
        assert!(b.y <= 1e-9, "over the top only");
        // born one after another the way round, left to right
        assert!(b.born == a.born + 1 && b.x > a.x);
    }
    let end = t.nodes.last().map_or((f64::NAN, 0.0), |n| (n.x, n.y));
    assert!(libm::hypot(end.0 - 20.0, end.1) < 1e-6, "{end:?}");
}
