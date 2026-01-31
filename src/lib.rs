use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{console, Document, Element, HtmlInputElement, Node};

/// CPMM state for a liquidity pool.
/// Uses the constant product invariant: x * y = k = L^2
/// where L is liquidity and P = y/x is the price.
#[derive(Clone, Copy, Debug)]
struct CpmmState {
    liquidity: f64,
    price: f64,
}

impl CpmmState {
    fn new(liquidity: f64, price: f64) -> Self {
        assert!(liquidity > 0.0, "Liquidity must be positive");
        assert!(price > 0.0, "Price must be positive");
        Self { liquidity, price }
    }

    /// Base reserves: x = L / sqrt(P)
    fn base_reserves(&self) -> f64 {
        self.liquidity / self.price.sqrt()
    }

    /// Quote reserves: y = L * sqrt(P)
    fn quote_reserves(&self) -> f64 {
        self.liquidity * self.price.sqrt()
    }

    /// Invariant k = L^2 = x * y
    #[allow(dead_code)]
    fn invariant(&self) -> f64 {
        self.liquidity * self.liquidity
    }
}

/// Computes wallet deltas and fee collection for a trade.
/// The trader moves the pool from initial_state to final_state.
/// Wallet deltas are from the trader's perspective (positive = received).
/// Fees are collected on the input side and sent to treasury.
#[derive(Clone, Copy, Debug)]
struct TradeResult {
    price_delta: f64,
    base_wallet_delta: f64,
    quote_wallet_delta: f64,
    base_fee_collected: f64,
    quote_fee_collected: f64,
}

impl TradeResult {
    fn compute(initial: CpmmState, final_state: CpmmState, fee_fraction: f64) -> Self {
        assert!(
            (0.0..1.0).contains(&fee_fraction),
            "Fee must be in [0, 1)"
        );

        let price_delta = final_state.price - initial.price;

        // Pool reserve changes
        let base_pool_delta = final_state.base_reserves() - initial.base_reserves();
        let quote_pool_delta = final_state.quote_reserves() - initial.quote_reserves();

        // Wallet deltas are opposite of pool deltas (what leaves pool enters wallet)
        // Before fees, gross amounts
        let base_gross = -base_pool_delta;
        let quote_gross = -quote_pool_delta;

        // Fee is collected on the input side (negative wallet delta means trader pays)
        // If trader pays base (base_gross < 0), fee is on base
        // If trader pays quote (quote_gross < 0), fee is on quote
        let (base_fee, quote_fee) = if base_gross < 0.0 {
            // Trader is selling base (paying base, receiving quote)
            let fee = (-base_gross) * fee_fraction;
            (fee, 0.0)
        } else if quote_gross < 0.0 {
            // Trader is buying base (paying quote, receiving base)
            let fee = (-quote_gross) * fee_fraction;
            (0.0, fee)
        } else {
            // No trade or edge case
            (0.0, 0.0)
        };

        // Net wallet deltas after fee deduction
        // Fee is deducted from what trader would receive, conceptually
        // But since fee is on input, the output is reduced by the fee's worth
        // For simplicity, we show fee as separate collection
        let base_wallet_delta = base_gross;
        let quote_wallet_delta = quote_gross;

        Self {
            price_delta,
            base_wallet_delta,
            quote_wallet_delta,
            base_fee_collected: base_fee,
            quote_fee_collected: quote_fee,
        }
    }
}

/// Converts a slider value in [0, 1] to a logarithmic price.
/// Maps 0.5 to the center price, with exponential scaling.
fn slider_to_price(slider_value: f64, center_price: f64, decades: f64) -> f64 {
    let exponent = (slider_value - 0.5) * 2.0 * decades;
    center_price * 10.0_f64.powf(exponent)
}

/// Converts a price to a slider value in [0, 1].
fn price_to_slider(price: f64, center_price: f64, decades: f64) -> f64 {
    if price <= 0.0 || center_price <= 0.0 {
        return 0.5;
    }
    let exponent = (price / center_price).log10();
    0.5 + exponent / (2.0 * decades)
}

/// Formats a number with appropriate precision.
fn format_number(value: f64) -> String {
    if value.abs() < 0.0001 && value != 0.0 {
        format!("{:.6e}", value)
    } else if value.abs() >= 1_000_000.0 {
        format!("{:.4e}", value)
    } else {
        format!("{:.6}", value)
    }
}

/// Shared application state.
struct AppState {
    initial_liquidity: f64,
    initial_price: f64,
    final_price: f64,
    fee_percent: f64,
    center_price: f64,
    decades: f64,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            initial_liquidity: 1000.0,
            initial_price: 1.0,
            final_price: 1.1,
            fee_percent: 0.3,
            center_price: 1.0,
            decades: 3.0,
        }
    }
}

type SharedState = Rc<RefCell<AppState>>;

/// Converts an Element to a Node reference for append operations.
fn as_node(element: &Element) -> &Node {
    element.as_ref()
}

/// Creates a labeled input row.
fn create_input_row(
    document: &Document,
    label1: &str,
    id1: &str,
    value1: &str,
    label2: Option<&str>,
    id2: Option<&str>,
    value2: Option<&str>,
) -> Result<Element, JsValue> {
    let row = document.create_element("div")?;
    row.set_attribute("class", "cpmm-row")?;

    let create_field = |label: &str, id: &str, value: &str| -> Result<Element, JsValue> {
        let field = document.create_element("div")?;
        field.set_attribute("class", "cpmm-field")?;

        let lbl = document.create_element("label")?;
        lbl.set_text_content(Some(label));
        lbl.set_attribute("for", id)?;

        let input = document.create_element("input")?;
        input.set_attribute("type", "text")?;
        input.set_attribute("id", id)?;
        input.set_attribute("value", value)?;

        field.append_child(as_node(&lbl))?;
        field.append_child(as_node(&input))?;
        Ok(field)
    };

    let field1 = create_field(label1, id1, value1)?;
    row.append_child(as_node(&field1))?;

    if let (Some(l2), Some(i2), Some(v2)) = (label2, id2, value2) {
        let field2 = create_field(l2, i2, v2)?;
        row.append_child(as_node(&field2))?;
    }

    Ok(row)
}

/// Creates a slider row.
fn create_slider_row(document: &Document, id: &str, value: f64) -> Result<Element, JsValue> {
    let row = document.create_element("div")?;
    row.set_attribute("class", "cpmm-slider-row")?;

    let label = document.create_element("label")?;
    label.set_text_content(Some("Logarithmic Price Slider"));

    let slider = document.create_element("input")?;
    slider.set_attribute("type", "range")?;
    slider.set_attribute("id", id)?;
    slider.set_attribute("min", "0")?;
    slider.set_attribute("max", "1")?;
    slider.set_attribute("step", "0.001")?;
    slider.set_attribute("value", &value.to_string())?;
    slider.set_attribute("class", "cpmm-slider")?;

    row.append_child(as_node(&label))?;
    row.append_child(as_node(&slider))?;
    Ok(row)
}

/// Creates a section with a title.
fn create_section(document: &Document, title: &str) -> Result<Element, JsValue> {
    let section = document.create_element("div")?;
    section.set_attribute("class", "cpmm-section")?;

    let header = document.create_element("div")?;
    header.set_attribute("class", "cpmm-section-header")?;
    header.set_text_content(Some(title));

    section.append_child(as_node(&header))?;
    Ok(section)
}

/// Gets an input element by ID.
fn get_input(document: &Document, id: &str) -> Option<HtmlInputElement> {
    document
        .get_element_by_id(id)
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
}

/// Sets the value of an input element.
fn set_input_value(document: &Document, id: &str, value: &str) {
    if let Some(input) = get_input(document, id) {
        input.set_value(value);
    }
}

/// Updates all computed fields based on current state.
fn update_computed_fields(document: &Document, state: &AppState) {
    let initial = CpmmState::new(state.initial_liquidity, state.initial_price);
    let final_state = CpmmState::new(state.initial_liquidity, state.final_price);
    let fee_fraction = state.fee_percent / 100.0;

    // Initial reserves
    set_input_value(
        document,
        "initial-base-reserves",
        &format_number(initial.base_reserves()),
    );
    set_input_value(
        document,
        "initial-quote-reserves",
        &format_number(initial.quote_reserves()),
    );

    // Final reserves
    set_input_value(
        document,
        "final-base-reserves",
        &format_number(final_state.base_reserves()),
    );
    set_input_value(
        document,
        "final-quote-reserves",
        &format_number(final_state.quote_reserves()),
    );

    // Trade result
    let result = TradeResult::compute(initial, final_state, fee_fraction);

    set_input_value(
        document,
        "delta-price",
        &format_number(result.price_delta),
    );
    set_input_value(
        document,
        "delta-base-reserves",
        &format_number(result.base_wallet_delta),
    );
    set_input_value(
        document,
        "delta-quote-reserves",
        &format_number(result.quote_wallet_delta),
    );
    set_input_value(
        document,
        "fee-base-collected",
        &format_number(result.base_fee_collected),
    );
    set_input_value(
        document,
        "fee-quote-collected",
        &format_number(result.quote_fee_collected),
    );
}

/// Attaches an input event listener to an element.
fn attach_input_listener<F>(document: &Document, id: &str, callback: F)
where
    F: Fn(String) + 'static,
{
    if let Some(input) = get_input(document, id) {
        let closure = Closure::wrap(Box::new(move |_event: web_sys::InputEvent| {
            let input_clone = input.clone();
            callback(input_clone.value());
        }) as Box<dyn Fn(_)>);
        let input_for_listener = get_input(document, id).unwrap();
        input_for_listener
            .add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

/// Main entry point for injecting the CPMM calculator UI.
#[wasm_bindgen]
pub fn inject_ui(anchor_id: &str) {
    console::log_1(&"CPMM Calculator: Initializing...".into());

    let window = match web_sys::window() {
        Some(w) => w,
        None => {
            console::error_1(&"No window object found".into());
            return;
        }
    };

    let document = match window.document() {
        Some(d) => d,
        None => {
            console::error_1(&"No document object found".into());
            return;
        }
    };

    let anchor = match document.get_element_by_id(anchor_id) {
        Some(a) => a,
        None => {
            console::error_1(&format!("Anchor element '{}' not found", anchor_id).into());
            return;
        }
    };

    if let Err(e) = build_ui(&document, &anchor) {
        console::error_1(&format!("Failed to build UI: {:?}", e).into());
    }
}

/// Builds the complete calculator UI.
fn build_ui(document: &Document, anchor: &Element) -> Result<(), JsValue> {
    let state: SharedState = Rc::new(RefCell::new(AppState::default()));

    let container = document.create_element("div")?;
    container.set_attribute("class", "cpmm-calculator")?;

    // Initial Price Section
    let initial_section = create_section(document, "Initial Price Section")?;

    let initial_slider_value = {
        let s = state.borrow();
        price_to_slider(s.initial_price, s.center_price, s.decades)
    };

    let row1 = create_input_row(
        document,
        "Liquidity:",
        "initial-liquidity",
        &format_number(state.borrow().initial_liquidity),
        Some("Price:"),
        Some("initial-price"),
        Some(&format_number(state.borrow().initial_price)),
    )?;
    initial_section.append_child(as_node(&row1))?;

    let slider1 = create_slider_row(document, "initial-price-slider", initial_slider_value)?;
    initial_section.append_child(as_node(&slider1))?;

    let row2 = create_input_row(
        document,
        "Base Reserves:",
        "initial-base-reserves",
        "",
        Some("Quote Reserves:"),
        Some("initial-quote-reserves"),
        Some(""),
    )?;
    initial_section.append_child(as_node(&row2))?;

    container.append_child(as_node(&initial_section))?;

    // Final Price Section
    let final_section = create_section(document, "Final Price Section")?;

    let final_slider_value = {
        let s = state.borrow();
        price_to_slider(s.final_price, s.center_price, s.decades)
    };

    let row3 = create_input_row(
        document,
        "Fee %:",
        "fee-percent",
        &format_number(state.borrow().fee_percent),
        Some("Price:"),
        Some("final-price"),
        Some(&format_number(state.borrow().final_price)),
    )?;
    final_section.append_child(as_node(&row3))?;

    let slider2 = create_slider_row(document, "final-price-slider", final_slider_value)?;
    final_section.append_child(as_node(&slider2))?;

    let row4 = create_input_row(
        document,
        "Base Reserves:",
        "final-base-reserves",
        "",
        Some("Quote Reserves:"),
        Some("final-quote-reserves"),
        Some(""),
    )?;
    final_section.append_child(as_node(&row4))?;

    container.append_child(as_node(&final_section))?;

    // Delta Section
    let delta_section = create_section(document, "Delta Section (Wallet Perspective)")?;

    let row5 = create_input_row(
        document,
        "",
        "delta-empty",
        "",
        Some("Price Delta:"),
        Some("delta-price"),
        Some(""),
    )?;
    delta_section.append_child(as_node(&row5))?;

    let row6 = create_input_row(
        document,
        "Base Reserves Delta:",
        "delta-base-reserves",
        "",
        Some("Quote Reserves Delta:"),
        Some("delta-quote-reserves"),
        Some(""),
    )?;
    delta_section.append_child(as_node(&row6))?;

    let row7 = create_input_row(
        document,
        "Base Fee Collected:",
        "fee-base-collected",
        "",
        Some("Quote Fee Collected:"),
        Some("fee-quote-collected"),
        Some(""),
    )?;
    delta_section.append_child(as_node(&row7))?;

    container.append_child(as_node(&delta_section))?;

    // Insert container before anchor
    if let Some(parent) = anchor.parent_node() {
        parent.insert_before(&container, Some(anchor))?;
    }

    // Initial computation
    update_computed_fields(document, &state.borrow());

    // Attach event listeners
    let doc = document.clone();
    let state_clone = Rc::clone(&state);
    attach_input_listener(document, "initial-liquidity", move |value| {
        if let Ok(v) = value.parse::<f64>()
            && v > 0.0
        {
            state_clone.borrow_mut().initial_liquidity = v;
            update_computed_fields(&doc, &state_clone.borrow());
        }
    });

    let doc = document.clone();
    let state_clone = Rc::clone(&state);
    attach_input_listener(document, "initial-price", move |value| {
        if let Ok(v) = value.parse::<f64>()
            && v > 0.0
        {
            {
                let mut s = state_clone.borrow_mut();
                s.initial_price = v;
            }
            let s = state_clone.borrow();
            let slider_val = price_to_slider(v, s.center_price, s.decades);
            set_input_value(&doc, "initial-price-slider", &slider_val.to_string());
            update_computed_fields(&doc, &s);
        }
    });

    let doc = document.clone();
    let state_clone = Rc::clone(&state);
    attach_input_listener(document, "initial-price-slider", move |value| {
        if let Ok(v) = value.parse::<f64>() {
            let price = {
                let s = state_clone.borrow();
                slider_to_price(v, s.center_price, s.decades)
            };
            state_clone.borrow_mut().initial_price = price;
            set_input_value(&doc, "initial-price", &format_number(price));
            update_computed_fields(&doc, &state_clone.borrow());
        }
    });

    let doc = document.clone();
    let state_clone = Rc::clone(&state);
    attach_input_listener(document, "fee-percent", move |value| {
        if let Ok(v) = value.parse::<f64>()
            && (0.0..100.0).contains(&v)
        {
            state_clone.borrow_mut().fee_percent = v;
            update_computed_fields(&doc, &state_clone.borrow());
        }
    });

    let doc = document.clone();
    let state_clone = Rc::clone(&state);
    attach_input_listener(document, "final-price", move |value| {
        if let Ok(v) = value.parse::<f64>()
            && v > 0.0
        {
            {
                let mut s = state_clone.borrow_mut();
                s.final_price = v;
            }
            let s = state_clone.borrow();
            let slider_val = price_to_slider(v, s.center_price, s.decades);
            set_input_value(&doc, "final-price-slider", &slider_val.to_string());
            update_computed_fields(&doc, &s);
        }
    });

    let doc = document.clone();
    let state_clone = Rc::clone(&state);
    attach_input_listener(document, "final-price-slider", move |value| {
        if let Ok(v) = value.parse::<f64>() {
            let price = {
                let s = state_clone.borrow();
                slider_to_price(v, s.center_price, s.decades)
            };
            state_clone.borrow_mut().final_price = price;
            set_input_value(&doc, "final-price", &format_number(price));
            update_computed_fields(&doc, &state_clone.borrow());
        }
    });

    console::log_1(&"CPMM Calculator: UI initialized successfully".into());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_cpmm_state_reserves() {
        // L = 100, P = 4
        // x = L / sqrt(P) = 100 / 2 = 50
        // y = L * sqrt(P) = 100 * 2 = 200
        let state = CpmmState::new(100.0, 4.0);
        assert!(approx_eq(state.base_reserves(), 50.0));
        assert!(approx_eq(state.quote_reserves(), 200.0));
    }

    #[test]
    fn test_cpmm_invariant() {
        // k = L^2 = x * y
        let state = CpmmState::new(100.0, 4.0);
        let k = state.invariant();
        let xy = state.base_reserves() * state.quote_reserves();
        assert!(approx_eq(k, xy));
        assert!(approx_eq(k, 10000.0));
    }

    #[test]
    fn test_price_from_reserves() {
        // P = y / x
        let state = CpmmState::new(100.0, 4.0);
        let computed_price = state.quote_reserves() / state.base_reserves();
        assert!(approx_eq(computed_price, state.price));
    }

    #[test]
    fn test_trade_result_buy_base() {
        // Initial: L=1000, P=1.0 => x=1000, y=1000
        // Final: L=1000, P=1.21 => x=909.09, y=1100
        // Trader buys base: receives ~90.91 base, pays ~100 quote
        let initial = CpmmState::new(1000.0, 1.0);
        let final_state = CpmmState::new(1000.0, 1.21);
        let result = TradeResult::compute(initial, final_state, 0.003);

        assert!(result.base_wallet_delta > 0.0); // Trader receives base
        assert!(result.quote_wallet_delta < 0.0); // Trader pays quote
        assert!(result.quote_fee_collected > 0.0); // Fee on quote input
        assert!(approx_eq(result.base_fee_collected, 0.0)); // No fee on base
    }

    #[test]
    fn test_trade_result_sell_base() {
        // Price decreases: trader sells base for quote
        let initial = CpmmState::new(1000.0, 1.0);
        let final_state = CpmmState::new(1000.0, 0.81);
        let result = TradeResult::compute(initial, final_state, 0.003);

        assert!(result.base_wallet_delta < 0.0); // Trader pays base
        assert!(result.quote_wallet_delta > 0.0); // Trader receives quote
        assert!(result.base_fee_collected > 0.0); // Fee on base input
        assert!(approx_eq(result.quote_fee_collected, 0.0)); // No fee on quote
    }

    #[test]
    fn test_slider_price_conversion_roundtrip() {
        let center = 1.0;
        let decades = 3.0;
        let prices = [0.001, 0.1, 1.0, 10.0, 100.0, 1000.0];

        for &price in &prices {
            let slider = price_to_slider(price, center, decades);
            let recovered = slider_to_price(slider, center, decades);
            assert!(
                (price - recovered).abs() / price < 0.001,
                "Roundtrip failed for price {}",
                price
            );
        }
    }

    #[test]
    fn test_slider_center() {
        let center = 10.0;
        let decades = 2.0;

        // Slider at 0.5 should give center price
        let price = slider_to_price(0.5, center, decades);
        assert!(approx_eq(price, center));
    }
}
