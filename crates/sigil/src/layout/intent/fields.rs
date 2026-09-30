use crate::layout::style_data::StyleData;
use crate::style::merge::{ResolvedSlot, SlotSet};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct LayoutFields(u64);

impl LayoutFields {
    pub(crate) const ALL: Self = Self(u64::MAX);
    pub(crate) const GRID_COLUMNS: Self = Self(1 << 44);
    pub(crate) const GRID_ROWS: Self = Self(1 << 45);

    pub(crate) const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    const fn has(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

macro_rules! fields {
    ($($bit:literal $name:ident [$($slot:ident)*] $($field:ident).+;)*) => {
        impl LayoutFields {
            $(pub(crate) const $name: Self = Self(1 << $bit);)*
        }

        pub(super) fn merge(to: &mut StyleData, from: &StyleData, set: LayoutFields) {
            $(if set.has(LayoutFields::$name) {
                to.$($field).+ = from.$($field).+;
            })*
            if set.has(LayoutFields::GRID_COLUMNS) {
                to.grid_template_columns.clone_from(&from.grid_template_columns);
            }
            if set.has(LayoutFields::GRID_ROWS) {
                to.grid_template_rows.clone_from(&from.grid_template_rows);
            }
        }

        pub(super) fn slots(set: LayoutFields) -> SlotSet {
            let table: &[(LayoutFields, &[ResolvedSlot])] = &[
                $((LayoutFields::$name, &[$(ResolvedSlot::$slot),*]),)*
                (LayoutFields::GRID_COLUMNS, &[ResolvedSlot::GridColumns]),
                (LayoutFields::GRID_ROWS, &[ResolvedSlot::GridRows]),
            ];
            table
                .iter()
                .filter(|(field, _)| set.has(*field))
                .flat_map(|(_, slots)| slots.iter())
                .fold(SlotSet::EMPTY, |acc, slot| acc.with(*slot))
        }
    };
}

fields! {
    0 DISPLAY [Display] display;
    1 POSITION [] position;
    2 OVERFLOW_X [OverflowX] overflow.width;
    3 OVERFLOW_Y [OverflowY] overflow.height;
    4 DIRECTION [FlexDirection] flex_direction;
    5 WRAP [FlexWrap] flex_wrap;
    6 GROW [FlexGrow] flex_grow;
    7 SHRINK [FlexShrink] flex_shrink;
    8 BASIS [] flex_basis;
    9 JUSTIFY [JustifyContent] justify_content;
    10 ALIGN_ITEMS [AlignItems] align_items;
    11 ALIGN_CONTENT [] align_content;
    12 ALIGN_SELF [] align_self;
    13 GAP_X [Gap GapX] gap.width;
    14 GAP_Y [Gap GapY] gap.height;
    15 MARGIN_TOP [MarginTop] margin.top;
    16 MARGIN_RIGHT [MarginRight] margin.right;
    17 MARGIN_BOTTOM [MarginBottom] margin.bottom;
    18 MARGIN_LEFT [MarginLeft] margin.left;
    19 INSET_TOP [] inset.top;
    20 INSET_RIGHT [] inset.right;
    21 INSET_BOTTOM [] inset.bottom;
    22 INSET_LEFT [] inset.left;
    23 PADDING_TOP [PaddingTop] padding.top;
    24 PADDING_RIGHT [PaddingRight] padding.right;
    25 PADDING_BOTTOM [PaddingBottom] padding.bottom;
    26 PADDING_LEFT [PaddingLeft] padding.left;
    27 BORDER_TOP [BorderTop BorderWidth] border.top;
    28 BORDER_RIGHT [BorderRight BorderWidth] border.right;
    29 BORDER_BOTTOM [BorderBottom BorderWidth] border.bottom;
    30 BORDER_LEFT [BorderLeft BorderWidth] border.left;
    31 WIDTH [Width] size.width;
    32 HEIGHT [Height] size.height;
    33 MIN_WIDTH [MinWidth] min_size.width;
    34 MIN_HEIGHT [MinHeight] min_size.height;
    35 MAX_WIDTH [MaxWidth] max_size.width;
    36 MAX_HEIGHT [MaxHeight] max_size.height;
    37 ORDER [] order;
    38 BOX_SIZING [] box_sizing;
    39 COLUMN_SPAN [] grid_column_span;
    40 ROW_SPAN [] grid_row_span;
    41 COLUMN_START [] grid_column_start;
    42 ROW_START [] grid_row_start;
    43 DENSE [] grid_auto_flow_dense;
}
