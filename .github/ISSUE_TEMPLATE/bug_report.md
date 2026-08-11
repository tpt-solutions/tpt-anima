name: Bug report
description: Report a defect in TPT Anima
title: "[BUG] "
labels: ["bug"]
body:
  - type: dropdown
    id: layer
    attributes:
      label: Affected layer
      options:
        - SSE (Semantic State Engine)
        - INP (Intent & Negotiation Protocol)
        - CK (Cognitive Kernel)
        - ZKPA (Zero-Knowledge Proof of Agency)
        - Edge Gateway
        - Observatory
        - Other / unknown
    validations:
      required: true
  - type: textarea
    id: what
    attributes:
      label: What happened?
      description: A clear and concise description of the bug.
    validations:
      required: true
  - type: textarea
    id: repro
    attributes:
      label: Steps to reproduce
      description: How can we reproduce the problem? Include commands, inputs, and expected vs. actual behavior.
      placeholder: |
        1. ...
        2. ...
    validations:
      required: true
  - type: textarea
    id: env
    attributes:
      label: Environment
      description: OS, Rust version (`rustc --version`), crate/commit, hardware tier.
    validations:
      required: false
