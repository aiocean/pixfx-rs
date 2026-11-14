// A map as a picture within a budget: the biggest whole scale that fits, grid lines only where cells are big
// enough to carry them, and an averaged overview when even one pixel a cell is too much.
use pixfx_pixel::view::{View, ViewOptions, view};
use pixfx_pixel::{Map, Rgba, zoom};

const RED: Rgba = Rgba(220, 30, 30, 255);
const RED_PX: [u8; 4] = [220, 30, 30, 255];

fn full(w: u32, h: u32) -> Map {
    Map {
        w,
        h,
        cells: vec![Some(0); (w * h) as usize],
    }
}

fn see(map: &Map, opts: &ViewOptions) -> View {
    let v = view(map, &[RED], opts);
    // the same picture twice: nothing in it depends on the run
    assert_eq!(view(map, &[RED], opts), v);
    v
}

#[test]
fn a_small_map_gets_the_largest_scale_with_grid_lines() {
    let map = full(16, 16);
    let v = see(&map, &ViewOptions::default());
    assert_eq!((v.scale, v.overview), (16, false));
    assert!(v.image.w <= 512 && v.image.h <= 512);
    // the same picture zoom draws: cells inside one-pixel lines
    assert_eq!(v.image, zoom(&map, &[RED], 16));
    assert_eq!((v.image.w, v.image.h), (16 * 17 + 1, 16 * 17 + 1));
    assert_ne!(v.image.get(0, 5), Some(RED_PX));
    assert_eq!(v.image.get(5, 5), Some(RED_PX));
}

#[test]
fn a_bigger_map_gets_the_largest_scale_that_fits_and_its_cells_touch() {
    // at 8 a cell would need grid lines: 64 * 9 + 1 = 577 > 512; at 7 the cells touch: 64 * 7 = 448
    let v = see(&full(64, 64), &ViewOptions::default());
    assert_eq!((v.scale, v.overview), (7, false));
    assert_eq!((v.image.w, v.image.h), (448, 448));
    // no grid lines: every pixel is the cell colour
    assert!(v.image.rgba.chunks(4).all(|p| p == RED_PX));
}

#[test]
fn grid_lines_come_only_from_scale_8() {
    let map = full(4, 4);
    let at = |max_scale| {
        see(
            &map,
            &ViewOptions {
                max_scale,
                ..ViewOptions::default()
            },
        )
    };
    let seven = at(7);
    assert_eq!((seven.scale, seven.image.w), (7, 28));
    assert!(seven.image.rgba.chunks(4).all(|p| p == RED_PX));
    let eight = at(8);
    assert_eq!((eight.scale, eight.image.w), (8, 4 * 9 + 1));
    assert_ne!(eight.image.get(0, 3), Some(RED_PX));
}

#[test]
fn a_map_too_big_for_the_budget_is_an_averaged_overview_without_grid_lines() {
    // the left half red, the right half blue: an overview pixel at the seam averages the two
    let blue = Rgba(30, 30, 220, 255);
    let (w, h) = (900u32, 900u32);
    let map = Map {
        w,
        h,
        cells: (0..w * h)
            .map(|i| Some(usize::from(i % w >= 451)))
            .collect(),
    };
    let v = view(&map, &[RED, blue], &ViewOptions::default());
    assert!(v.overview);
    assert_eq!(v.scale, 2);
    assert!(v.image.w <= 512 && v.image.h <= 512);
    assert_eq!((v.image.w, v.image.h), (450, 450));
    assert_eq!(v.image.get(0, 0), Some(RED_PX));
    assert_eq!(v.image.get(449, 449), Some([30, 30, 220, 255]));
    // cells 450 (red) and 451 (blue) share pixel 225
    assert_eq!(v.image.get(225, 0), Some([125, 30, 125, 255]));
}

#[test]
fn a_region_is_cut_out_before_it_is_scaled() {
    // a 900x900 map is an overview; its 10x10 corner seen alone is big and gridded
    let mut map = Map::empty(900, 900);
    if let Some(c) = map.cells.get_mut(900 + 1) {
        *c = Some(0);
    }
    let v = see(
        &map,
        &ViewOptions {
            region: Some([0, 0, 10, 10]),
            ..ViewOptions::default()
        },
    );
    assert_eq!((v.scale, v.overview), (16, false));
    assert_eq!((v.image.w, v.image.h), (10 * 17 + 1, 10 * 17 + 1));
    // cell (1, 1) is the painted one
    assert_eq!(v.image.get(17 + 8, 17 + 8), Some(RED_PX));
    assert_ne!(v.image.get(8, 8), Some(RED_PX));
    // a region past the map's edge is clipped to it
    let edge = see(
        &map,
        &ViewOptions {
            region: Some([895, 895, 10, 10]),
            ..ViewOptions::default()
        },
    );
    assert_eq!(edge.image.w, 5 * 17 + 1);
}

#[test]
fn brighter_lines_follow_every_counted_from_the_map_not_the_region() {
    let map = full(12, 1);
    let line = |region, every| {
        let v = see(
            &map,
            &ViewOptions {
                region,
                every,
                ..ViewOptions::default()
            },
        );
        // the vertical lines along the middle of the row, one per cell boundary
        (0..=v.image.w / 17)
            .map(|i| v.image.get(i * 17, 8))
            .collect::<Vec<_>>()
    };
    let whole = line(None, 4);
    assert!(whole.first() == whole.get(4) && whole.get(4) != whole.get(1));
    // from column 2 on, the brighter line sits at map column 4: the region's line 2
    let part = line(Some([2, 0, 10, 1]), 4);
    assert!(part.get(2) == whole.first() && part.first() != part.get(2));
}
