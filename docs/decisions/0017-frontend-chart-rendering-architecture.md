# 17: Frontend Chart Architecture


- Status: Accepted

## Context

When designing portfolio visualizations, having disjoint chart components with their own canvases, paddings, axis styles, and timeline calculations leads to a fragmented UI.
Furthermore, mixing complex financial calculations (e.g., currency-converted values, benchmark matching, drawdowns, rolling volatility, and performance contributions) with UI plotting logic spreads domain decisions across layers, making the app harder to maintain and test.

## Decision

We decouple visual representation from analytical logic using a strict architecture:

1. **Unified Layout Frame (Chart.tsx)**: All charts share a single structural shell containing the legend, interactive coordinate readout, coordinate grid, right-aligned value axis, and bottom timeline axis. Individual chart components only render their specific data paths or shapes within this frame.
2. **Real-Pixel Sizing**: Charts are rendered based on actual measured layout dimensions rather than scaled SVG viewBox attributes. This guarantees that fonts, gridlines, and stroke weights remain crisp and consistent across all screen sizes (desktop, tablet, mobile).
3. **Zero-Computation Frontend**: All financial calculations are evaluated in the Rust core kernel. The frontend is strictly a rendering layer that projects pre-calculated series onto 2D space. The only exceptions are width-dependent geometric computations (e.g., histogram binning, treemap layouts, and text truncation/wrapping).
4. **O(1) Crosshair Pointer Interaction**: Since dates are uniformly distributed, interactive hover detection computes coordinates directly using index-based scale division rather than walking the data array, ensuring smooth performance even with thousands of days.
