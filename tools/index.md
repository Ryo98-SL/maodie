# Tools Module

## Purpose

The tools module contains local repository maintenance scripts.

## Current Directory Structure

- `style-guard.mjs`: validates the initial documentation and workspace skeleton.

## Key Behaviors

Scripts here are invoked from root package scripts and should avoid project-specific business logic.

## Integration Notes

When a script becomes package-specific, move it into that package and expose it through the package's Nx targets.
