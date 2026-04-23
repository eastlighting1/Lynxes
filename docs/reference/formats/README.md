# Format Reference

This section documents the file and graph-shape conventions Lynxes expects when a graph enters or leaves the engine. The pages here are narrower than the authoring guides. They focus on exact field meanings, structural constraints, and validation expectations rather than on teaching someone how to write a first example.

## Available Pages

- [Reserved graph columns](reserved-columns.md)
- [`.gf` format](gf.md)
- [`.gf` authoring guide](../../gf_authoring_guide.md)
- [`.gfb` format](gfb.md)
- [Parquet graph shape](parquet-interop.md)

## How The `.gf` Pages Fit Together

The `.gf` material now lives in two companion pages because they serve different jobs.

Use [`.gf` format](gf.md) when you want the compact reference version: accepted edge forms, property placement, and validation-oriented reminders.

Use [`.gf` authoring guide](../../gf_authoring_guide.md) when you are actually writing a file from scratch and want the broader authoring flow, schema examples, metadata blocks, supported literal types, and style guidance in one place.

## Scope

Use these pages when the question is format-specific: which reserved columns are required, where edge properties belong in `.gf`, what `.gfb` is good for, or what shape a parquet graph loader expects. If you are learning the file formats from scratch, the `.gf` authoring guide and the beginner loading guides are the better entry points.
