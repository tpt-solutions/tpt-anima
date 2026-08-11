name: Pull request
description: Describe your change for review
title: "[PR] "
labels: []
body:
  - type: textarea
    id: summary
    attributes:
      label: Summary
      description: What does this PR do and why?
    validations:
      required: true
  - type: checkboxes
    id: checks
    attributes:
      label: Checks
      options:
        - label: "`cargo fmt --all -- --check` passes"
        - label: "`cargo clippy --workspace --all-targets -- -D warnings` passes"
        - label: "`cargo test --workspace` passes"
        - label: "Documentation updated where relevant"
    validations:
      required: true
  - type: textarea
    id: testing
    attributes:
      label: Testing
      description: How was this verified? Include commands run and results.
    validations:
      required: false
