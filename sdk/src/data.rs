use crate::{
    DataError, DataTableOptions, DataTableState, Gui, NumericEdit, Scope, Selection, VisibleRows,
};

impl Gui {
    pub fn begin_rows(&mut self, label: &str, count: usize, height: f32) -> Result<(), DataError> {
        let token = if self.emitting() {
            Some(self.ctx.ui().begin_rows(label, count, height)?)
        } else {
            None
        };
        self.scopes.push(Scope::Rows(token));
        Ok(())
    }
    pub fn visible_rows(&self) -> VisibleRows {
        match self.scopes.last() {
            Some(Scope::Rows(Some(rows))) => rows.visible_rows(),
            Some(Scope::DataTable(Some(table))) => table.visible_rows(),
            Some(Scope::Rows(None) | Scope::DataTable(None)) => VisibleRows::default(),
            _ => panic!("visible_rows requires rows or data table"),
        }
    }
    pub fn begin_row(&mut self, index: usize) -> bool {
        if !self.emitting() {
            return false;
        }
        let Some(Scope::Rows(Some(rows))) = self.scopes.last() else {
            panic!("begin_row requires rows");
        };
        let Some(cell) = self.ctx.ui().begin_row(rows, index) else {
            return false;
        };
        self.scopes.push(Scope::DataCell(Some(cell)));
        true
    }
    pub fn end_rows(&mut self) {
        match self.scopes.pop() {
            Some(Scope::Rows(Some(s))) => self.ctx.ui().end_rows(s),
            Some(Scope::Rows(None)) => {}
            _ => panic!("end_rows crossed scope"),
        }
    }
    pub fn multi_select_list_with<'a>(
        &mut self,
        label: &str,
        count: usize,
        item_at: impl FnMut(usize) -> Option<&'a str>,
        selection: &mut Selection,
        size: [f32; 2],
    ) -> Option<bool> {
        if self.emitting() {
            self.ctx
                .ui()
                .multi_select_list_with(label, count, item_at, selection, size)
        } else {
            Some(false)
        }
    }
    pub fn multi_select_list(
        &mut self,
        label: &str,
        items: &[&str],
        selection: &mut Selection,
        size: [f32; 2],
    ) -> bool {
        self.multi_select_list_with(label, items.len(), |i| Some(items[i]), selection, size)
            .unwrap_or(false)
    }
    pub fn begin_data_table(
        &mut self,
        label: &str,
        headers: &[&str],
        rows: usize,
        state: &mut DataTableState,
        options: DataTableOptions,
    ) -> Result<(), DataError> {
        let token = if self.emitting() {
            Some(
                self.ctx
                    .ui()
                    .begin_data_table(label, headers, rows, state, options)?,
            )
        } else {
            None
        };
        self.scopes.push(Scope::DataTable(token));
        Ok(())
    }
    pub fn begin_data_table_cell(&mut self, row: usize, column: usize) -> bool {
        if !self.emitting() {
            return false;
        }
        let Some(Scope::DataTable(Some(table))) = self.scopes.last() else {
            panic!("cell requires data table");
        };
        let Some(cell) = self.ctx.ui().begin_data_table_cell(table, row, column) else {
            return false;
        };
        self.scopes.push(Scope::DataCell(Some(cell)));
        true
    }
    pub fn end_data_cell(&mut self) {
        match self.scopes.pop() {
            Some(Scope::DataCell(Some(s))) => self.ctx.ui().end_data_cell(s),
            Some(Scope::DataCell(None)) => {}
            _ => panic!("end_data_cell crossed scope"),
        }
    }
    pub fn end_data_table(&mut self) -> Option<f32> {
        match self.scopes.pop() {
            Some(Scope::DataTable(Some(s))) => Some(self.ctx.ui().end_data_table(s)),
            Some(Scope::DataTable(None)) => None,
            _ => panic!("end_data_table crossed scope"),
        }
    }
    pub fn input_float(
        &mut self,
        label: &str,
        value: &mut f32,
        edit: &mut NumericEdit,
        step: f32,
        min: f32,
        max: f32,
    ) -> Result<bool, DataError> {
        if self.emitting() {
            self.ctx
                .ui()
                .input_float(label, value, edit, step, min, max)
        } else {
            Ok(false)
        }
    }
    pub fn input_int(
        &mut self,
        label: &str,
        value: &mut i32,
        edit: &mut NumericEdit,
        step: i32,
        min: i32,
        max: i32,
    ) -> Result<bool, DataError> {
        if self.emitting() {
            self.ctx.ui().input_int(label, value, edit, step, min, max)
        } else {
            Ok(false)
        }
    }
    pub fn begin_menu_bar(&mut self, label: &str) {
        let scope = self.emitting().then(|| self.ctx.ui().begin_menu_bar(label));
        self.scopes.push(Scope::MenuBar(scope));
    }
    pub fn begin_menu(&mut self, label: &str, size: [f32; 2]) -> Result<bool, DataError> {
        if !self.emitting() {
            return Ok(false);
        }
        let Some(Scope::MenuBar(Some(bar))) = self.scopes.last_mut() else {
            panic!("menu requires menu bar");
        };
        let open = self.ctx.ui().begin_menu(bar, label, size)?;
        if open {
            self.scopes.push(Scope::DataMenu);
        }
        Ok(open)
    }
    pub fn begin_submenu(&mut self, label: &str, size: [f32; 2]) -> Result<bool, DataError> {
        if !self.emitting() {
            return Ok(false);
        }
        assert!(
            matches!(self.scopes.last(), Some(Scope::DataMenu)),
            "submenu requires menu"
        );
        let open = self.ctx.ui().begin_submenu(label, size)?;
        if open {
            self.scopes.push(Scope::DataMenu);
        }
        Ok(open)
    }
    pub fn menu_item(&mut self, label: &str, hint: &str, enabled: bool) -> bool {
        if !self.emitting() {
            return false;
        }
        assert!(
            matches!(self.scopes.last(), Some(Scope::DataMenu)),
            "item requires menu"
        );
        let bar = self
            .scopes
            .iter()
            .rev()
            .find_map(|s| {
                if let Scope::MenuBar(Some(bar)) = s {
                    Some(bar)
                } else {
                    None
                }
            })
            .expect("menu bar");
        self.ctx.ui().menu_item(bar, label, hint, enabled)
    }
    pub fn end_menu(&mut self) {
        match self.scopes.pop() {
            Some(Scope::DataMenu) => self.ctx.ui().end_menu(),
            _ => panic!("end_menu crossed scope"),
        }
    }
    pub fn end_menu_bar(&mut self) {
        match self.scopes.pop() {
            Some(Scope::MenuBar(Some(s))) => self.ctx.ui().end_menu_bar(s),
            Some(Scope::MenuBar(None)) => {}
            _ => panic!("end_menu_bar crossed scope"),
        }
    }
    pub fn begin_property_grid(
        &mut self,
        label: &str,
        label_width: f32,
        row_height: f32,
    ) -> Result<(), DataError> {
        let scope = if self.emitting() {
            Some(
                self.ctx
                    .ui()
                    .begin_property_grid(label, label_width, row_height)?,
            )
        } else {
            None
        };
        self.scopes.push(Scope::PropertyGrid(scope));
        Ok(())
    }
    pub fn begin_property_row(&mut self, label: &str) {
        let scope = if self.emitting() {
            let Some(Scope::PropertyGrid(Some(grid))) = self.scopes.last() else {
                panic!("row requires property grid");
            };
            Some(self.ctx.ui().begin_property_row(grid, label))
        } else {
            None
        };
        self.scopes.push(Scope::PropertyRow(scope));
    }
    pub fn end_property_row(&mut self) {
        match self.scopes.pop() {
            Some(Scope::PropertyRow(Some(s))) => self.ctx.ui().end_property_row(s),
            Some(Scope::PropertyRow(None)) => {}
            _ => panic!("end_property_row crossed scope"),
        }
    }
    pub fn end_property_grid(&mut self) {
        match self.scopes.pop() {
            Some(Scope::PropertyGrid(Some(s))) => self.ctx.ui().end_property_grid(s),
            Some(Scope::PropertyGrid(None)) => {}
            _ => panic!("end_property_grid crossed scope"),
        }
    }
}

impl Gui {
    pub fn plot(
        &mut self,
        label: &str,
        samples: &[f32],
        options: crate::PlotOptions,
    ) -> Result<Option<crate::PlotHover>, DataError> {
        if !self.emitting() {
            return Ok(None);
        }
        self.ctx.ui().plot(label, samples, options)
    }
    pub fn plot_with(
        &mut self,
        label: &str,
        count: usize,
        get: impl Fn(usize) -> f32,
        options: crate::PlotOptions,
    ) -> Result<Option<crate::PlotHover>, DataError> {
        if !self.emitting() {
            return Ok(None);
        }
        self.ctx.ui().plot_with(label, count, get, options)
    }
    pub fn sparkline(
        &mut self,
        label: &str,
        samples: &[f32],
        size: [f32; 2],
        color: [u8; 4],
    ) -> Result<Option<crate::PlotHover>, DataError> {
        if !self.emitting() {
            return Ok(None);
        }
        self.ctx.ui().sparkline(label, samples, size, color)
    }
    pub fn curve_editor(
        &mut self,
        label: &str,
        model: &mut crate::CurveEditor,
        size: [f32; 2],
    ) -> Result<bool, DataError> {
        if !self.emitting() {
            return Ok(false);
        }
        self.ctx.ui().curve_editor(label, model, size)
    }
    pub fn gradient_editor(
        &mut self,
        label: &str,
        model: &mut crate::GradientEditor,
        size: [f32; 2],
    ) -> Result<bool, DataError> {
        if !self.emitting() {
            return Ok(false);
        }
        self.ctx.ui().gradient_editor(label, model, size)
    }
}
