//! Starter cards use logical screen units. Only reserve navigation space when needed.
pub struct StarterGrid {
    pub cols: usize,
    pub rows: usize,
    pub per_page: usize,
    pub pages: usize,
}
impl StarterGrid {
    pub fn fit(width: f32, height: f32, count: usize) -> Self {
        let gap = 8.;
        let max_cols = (((width + gap) / (132. + gap)) as usize).clamp(1, count);
        let rows_that_fit = |h: f32| (((h + gap) / (96. + gap)) as usize).max(1);
        let paginate = max_cols * rows_that_fit(height) < count;
        let height = if paginate { height - 50. } else { height };
        let capacity = max_cols * rows_that_fit(height);
        let per_page = capacity.min(count);
        let mut best = Self {
            cols: max_cols,
            rows: per_page.div_ceil(max_cols),
            per_page,
            pages: count.div_ceil(per_page),
        };
        let mut best_photo = 0.;
        for cols in 1..=max_cols {
            let rows = per_page.div_ceil(cols);
            if rows > rows_that_fit(height) {
                continue;
            }
            let w = (width + gap) / cols as f32 - gap;
            let h = (height + gap) / rows as f32 - gap;
            let photo = (w - 8.).min((h - 30.) * 480. / 260.);
            if photo > best_photo {
                best_photo = photo;
                best.cols = cols;
                best.rows = rows;
            }
        }
        best
    }
}
