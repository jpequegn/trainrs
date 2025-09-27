# Sports Science Background & Methodology

## Overview

TrainRS implements evidence-based sports science methodologies for training load quantification and analysis. This guide provides the theoretical foundation and practical applications of the metrics calculated by TrainRS.

## Fundamental Concepts

### Training Load Theory

Training load represents the cumulative stress imposed on an athlete's physiological systems during exercise. It encompasses both the **intensity** and **duration** of training stimuli, providing a quantifiable measure of training stress.

**Key Principles:**
- **Dose-Response Relationship**: Training adaptations are proportional to the training stimulus
- **Specificity**: Training adaptations are specific to the imposed demands
- **Progressive Overload**: Gradual increase in training load drives continued adaptation
- **Recovery**: Adequate recovery allows for adaptation and prevents overreaching

### Physiological Foundations

#### Energy Systems
1. **Phosphocreatine System** (0-10 seconds)
   - Immediate energy for high-intensity efforts
   - Limited storage capacity

2. **Glycolytic System** (10 seconds - 2 minutes)
   - Anaerobic glucose metabolism
   - Produces lactate as byproduct

3. **Oxidative System** (2+ minutes)
   - Aerobic metabolism of carbohydrates and fats
   - Primary system for endurance exercise

#### Training Zones
Training zones represent different physiological intensity domains:

- **Zone 1 (Active Recovery)**: Below aerobic threshold
- **Zone 2 (Aerobic Base)**: Aerobic threshold to first lactate threshold
- **Zone 3 (Tempo)**: First to second lactate threshold
- **Zone 4 (Lactate Threshold)**: Second lactate threshold
- **Zone 5 (VO₂ Max)**: Above lactate threshold

## Core Metrics Implementation

### Training Stress Score (TSS)

**Formula:** `TSS = (Duration × Normalized Power × Intensity Factor) / (FTP × 3600) × 100`

**Scientific Basis:**
TSS quantifies training stress by combining intensity and duration into a single metric. Originally developed by Dr. Andy Coggan for cycling, the concept has been adapted for other sports.

**Interpretation:**
- **< 150**: Low stress session
- **150-300**: Moderate stress session
- **300-450**: High stress session
- **> 450**: Very high stress session

**Sport-Specific Adaptations:**
- **Cycling**: Based on power output (watts)
- **Running**: Based on pace and heart rate
- **Swimming**: Based on pace and perceived effort

### Intensity Factor (IF)

**Formula:** `IF = Normalized Power / FTP`

**Scientific Basis:**
Intensity Factor represents the ratio of effort relative to an athlete's functional threshold. It normalizes intensity across different session durations.

**Interpretation:**
- **< 0.75**: Recovery/aerobic intensity
- **0.75-0.85**: Tempo/threshold intensity
- **0.85-0.95**: Lactate threshold intensity
- **0.95-1.05**: VO₂ max intensity
- **> 1.05**: Neuromuscular power intensity

### Normalized Power (NP)

**Formula:** Complex 4-step algorithm accounting for physiological response time

**Scientific Basis:**
Normalized Power accounts for the non-linear relationship between power output and physiological stress. It provides a better estimate of metabolic cost than average power for variable-intensity efforts.

**Key Features:**
- **30-second rolling average**: Accounts for physiological response time
- **Fourth power weighting**: Reflects exponential stress increase at high intensities
- **Variability adjustment**: Higher for variable vs. steady efforts

### Performance Management Chart (PMC)

#### Chronic Training Load (CTL)
**Formula:** Exponentially Weighted Moving Average (42-day time constant)

**Scientific Basis:**
CTL represents fitness or training capacity. The 42-day time constant reflects the typical adaptation timeframe for aerobic fitness improvements.

**Interpretation:**
- **Increasing CTL**: Building fitness
- **Stable CTL**: Maintaining fitness
- **Decreasing CTL**: Detraining

#### Acute Training Load (ATL)
**Formula:** Exponentially Weighted Moving Average (7-day time constant)

**Scientific Basis:**
ATL represents fatigue or recent training stress. The 7-day time constant captures the timeframe for acute physiological recovery.

#### Training Stress Balance (TSB)
**Formula:** `TSB = CTL - ATL`

**Scientific Basis:**
TSB indicates the balance between fitness and fatigue, providing insight into readiness to perform or absorb additional training.

**Interpretation:**
- **Positive TSB**: Fresh, ready for hard training or competition
- **Near Zero TSB**: Balanced state, moderate training possible
- **Negative TSB**: Fatigued, focus on recovery

## Periodization Models

### Linear Periodization
- **Progressive overload**: Gradual increase in training load
- **Systematic variation**: Planned changes in volume and intensity
- **Peak preparation**: Specific preparation for competition

### Block Periodization
- **Concentrated loading**: Focus on specific training qualities
- **Sequential development**: Build fitness components in sequence
- **Rapid realization**: Quick conversion to competition performance

### Polarized Training
- **80/20 Distribution**: 80% easy, 20% hard training
- **Intensity zones**: Clear separation between easy and hard
- **Volume emphasis**: High volume at low intensity

## Zone Calculation Methods

### Power Zones (Cycling)
Based on Functional Threshold Power (FTP):
- **Zone 1**: < 55% FTP (Active Recovery)
- **Zone 2**: 56-75% FTP (Endurance)
- **Zone 3**: 76-90% FTP (Tempo)
- **Zone 4**: 91-105% FTP (Lactate Threshold)
- **Zone 5**: 106-120% FTP (VO₂ Max)
- **Zone 6**: 121-150% FTP (Anaerobic Capacity)
- **Zone 7**: > 150% FTP (Neuromuscular Power)

### Heart Rate Zones
Based on Lactate Threshold Heart Rate (LTHR):
- **Zone 1**: < 81% LTHR (Active Recovery)
- **Zone 2**: 81-89% LTHR (Aerobic Base)
- **Zone 3**: 90-93% LTHR (Aerobic)
- **Zone 4**: 94-99% LTHR (Lactate Threshold)
- **Zone 5**: 100-102% LTHR (VO₂ Max)
- **Zone 6**: 103-106% LTHR (Anaerobic)

### Pace Zones (Running)
Based on threshold pace:
- **Zone 1**: > 129% threshold pace (Easy)
- **Zone 2**: 114-129% threshold pace (Marathon)
- **Zone 3**: 106-113% threshold pace (Tempo)
- **Zone 4**: 97-105% threshold pace (Threshold)
- **Zone 5**: 90-96% threshold pace (VO₂ Max)

## Multi-Sport Considerations

### Training Load Distribution
Different sports stress different physiological systems:
- **Cycling**: Primarily muscular, lower eccentric loading
- **Running**: High eccentric loading, greater musculoskeletal stress
- **Swimming**: Upper body emphasis, technique-dependent

### Load Scaling Factors
TrainRS applies sport-specific scaling to account for:
- **Mechanical stress differences**
- **Recovery time variations**
- **Injury risk factors**

## Validation & Research

### Scientific Foundation
TrainRS metrics are based on peer-reviewed research:
- **Coggan & Allen (2006)**: Original TSS and PMC concepts
- **Seiler (2010)**: Polarized training distribution
- **Laursen & Jenkins (2002)**: Training intensity distribution

### Data Quality Standards
- **Precision arithmetic**: Rust Decimal prevents floating-point errors
- **Validated algorithms**: Implementation matches published formulas
- **Edge case handling**: Robust handling of incomplete or irregular data

## Practical Applications

### Training Planning
1. **Baseline Testing**: Establish threshold values
2. **Zone Setting**: Configure personalized training zones
3. **Load Progression**: Plan systematic training progression
4. **Recovery Scheduling**: Balance stress and recovery

### Performance Analysis
1. **Trend Analysis**: Track fitness and fatigue trends
2. **Session Evaluation**: Assess individual workout quality
3. **Adaptation Monitoring**: Evaluate training effectiveness
4. **Competition Preparation**: Optimize taper and peak

### Health Monitoring
1. **Overreaching Detection**: Identify excessive training stress
2. **Recovery Assessment**: Monitor adaptation and recovery
3. **Injury Prevention**: Manage training load progression
4. **Return to Training**: Systematic load progression after illness/injury

## References

1. Coggan, A. R., & Allen, H. (2006). *Training and Racing with a Power Meter*. VeloPress.
2. Seiler, S. (2010). What is best practice for training intensity and duration distribution in endurance athletes? *International Journal of Sports Physiology and Performance*, 5(3), 276-291.
3. Laursen, P. B., & Jenkins, D. G. (2002). The scientific basis for high-intensity interval training. *Sports Medicine*, 32(1), 53-73.
4. Friel, J. (2015). *The Power Meter Handbook*. VeloPress.
5. McGregor, S. J., Weese, R. K., & Ratz, I. K. (2009). Performance modeling in an Olympic 1500-m finalist. *Medicine & Science in Sports & Exercise*, 41(1), 99-105.
6. Banister, E. W. (1991). Modeling elite athletic performance. *Physiological Testing of Elite Athletes*, 347-424.

---

*For specific implementation details, see the [Training Load Guide](training-load.md) and [CLI Reference](cli-reference.md).*