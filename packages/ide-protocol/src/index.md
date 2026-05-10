# IDE Protocol Source Module

## Purpose

The source module exports shared protocol types used by IDE clients and language tools.

## Current Directory Structure

- `index.ts`: public protocol type definitions.

## Key Behaviors

The protocol currently models editor documents and compile requests/responses.

## Integration Notes

Keep browser, worker, and transport-specific behavior outside this package.
