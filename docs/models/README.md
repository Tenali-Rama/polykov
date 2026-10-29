# Model experiments

Spec §5 says every ML component is chosen by experiment. Each component gets one file here, `<component>.md`, containing:

- **Targets**: copied from spec §5.
- **Benchmark**: the dataset, how it was built, and the cold-start and long-history cases.
- **Candidates**: always includes a simple baseline.
- **Results table**: accuracy/calibration metric, latency, RAM, model size.
- **Decision**: the winner and why.
- **Re-run log**: date, and what changed since the last run.
