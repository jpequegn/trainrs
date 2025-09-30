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

## Advanced Power Analysis

### Critical Power (CP) and W' (W Prime)

**Scientific Basis:**
The Critical Power model describes the hyperbolic relationship between power output and time to exhaustion. It divides work capacity into two components:
- **Critical Power (CP)**: The highest sustainable aerobic power (watts)
- **W' (W prime)**: Finite anaerobic work capacity above CP (joules)

**Formula:** `P = CP + W'/t` (2-parameter hyperbolic model)

Where:
- P = Power output (watts)
- t = Time to exhaustion (seconds)
- CP = Critical Power (watts)
- W' = Anaerobic work capacity (joules)

**Key Concepts:**
1. **CP as Sustainable Power**: Power at or below CP can theoretically be sustained indefinitely
2. **W' as Anaerobic Battery**: Limited capacity that depletes above CP and recovers below CP
3. **Model Types**:
   - **2-Parameter Hyperbolic**: Standard model, linear regression on P vs 1/t
   - **3-Parameter Hyperbolic**: Extended model with time constant
   - **Linear P-1/t**: Mathematically equivalent to 2-parameter

**Interpretation:**
- **CP ≈ 1.00-1.05 × FTP**: CP is typically slightly higher than FTP
- **W' Range**: 15,000-25,000 joules for trained cyclists
- **R² > 0.95**: Indicates good model fit quality

### W' Balance Tracking

**Formula:** Dynamic balance calculation throughout workout

**Depletion (Power > CP):**
```
dW'/dt = -(Power - CP)
```

**Recovery (Power < CP):**
```
dW'/dt = (W'max - W') / τ
```
Where τ (tau) is the recovery time constant.

**Practical Applications:**
1. **Interval Training**: Monitor W' depletion during high-intensity efforts
2. **Pacing Strategy**: Prevent premature W' depletion in races
3. **Recovery Analysis**: Assess W' reconstitution between intervals
4. **Training Prescription**: Design workouts targeting specific W' depletion

**TrainRS Implementation:**
- Real-time W' balance calculation throughout workouts
- Minimum balance detection (most depleted point)
- Time spent below zero (anaerobic deficit)
- Simplified linear recovery model for computational efficiency

### Time-to-Exhaustion Prediction

**Formula:** `t = W' / (P - CP)`

**Applications:**
1. **Race Pacing**: Predict sustainable duration at target powers
2. **Interval Design**: Calculate work-to-rest ratios
3. **Breakthrough Prediction**: Estimate when W' will be fully depleted
4. **Current State Assessment**: Factor in partially depleted W' mid-workout

**Interpretation:**
- **Power = CP**: Infinite time (sustainable)
- **Power > CP**: Finite time based on W' available
- **Mid-Workout**: Adjust predictions based on current W' balance

### Mean Maximal Power (MMP) Curve

**Definition:** The maximum average power sustained over any given duration from 1 second to full workout length.

**Calculation Method:**
1. For each duration D (1s, 5s, 15s, 30s, 1min, 5min, 20min, etc.)
2. Calculate rolling average power over D seconds
3. Extract maximum value for that duration
4. Aggregate best efforts across multiple workouts

**Standard Duration Benchmarks:**
- **5 seconds**: Neuromuscular power (sprint)
- **1 minute**: Anaerobic capacity
- **5 minutes**: VO₂ max power
- **20 minutes**: Functional Threshold Power (FTP) approximation
- **60 minutes**: Hour power (FTP validation)

**Applications:**
1. **Power Profile Analysis**: Identify strengths (sprinter vs. time trialist)
2. **CP Model Input**: Use MMP values to calculate Critical Power
3. **Training Zone Setting**: Calibrate zones based on actual capabilities
4. **Performance Tracking**: Monitor improvements across all durations
5. **FTP Estimation**: 95% of 20-minute MMP approximates FTP

**TrainRS Implementation:**
- Automatic MMP calculation from workout data
- Multi-workout aggregation for season-best power curve
- Standard duration extraction for CP model fitting
- Power profile visualization and analysis

### Practical CP/W' Applications

#### Training Prescription
1. **Threshold Development**: Target CP power for sustained efforts
2. **Anaerobic Capacity**: Deplete W' to specific levels
3. **Neuromuscular Power**: Efforts well above CP for short durations
4. **Recovery Quality**: Monitor W' recovery rates

#### Race Pacing
1. **Sustainable Power**: Stay near CP for steady-state events
2. **Surge Management**: Track W' depletion during attacks
3. **Finishing Strategy**: Reserve W' for final efforts
4. **Terrain Adaptation**: Use W' for climbs, recover on descents

#### Performance Testing
1. **3-Minute All-Out Test**: Single maximal effort protocol
2. **Multi-Point Protocol**: 3-5 maximal efforts (3min, 8min, 12min)
3. **MMP-Derived**: Extract from training data automatically
4. **Model Validation**: R² values assess fit quality

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
7. Monod, H., & Scherrer, J. (1965). The work capacity of a synergic muscular group. *Ergonomics*, 8(3), 329-338.
8. Morton, R. H. (1996). A 3-parameter critical power model. *Ergonomics*, 39(4), 611-619.
9. Jones, A. M., Vanhatalo, A., Burnley, M., Morton, R. H., & Poole, D. C. (2010). Critical power: Implications for determination of VO₂max and exercise tolerance. *Medicine & Science in Sports & Exercise*, 42(10), 1876-1890.
10. Skiba, P. F., Chidnok, W., Vanhatalo, A., & Jones, A. M. (2012). Modeling the expenditure and reconstitution of work capacity above critical power. *Medicine & Science in Sports & Exercise*, 44(8), 1526-1532.

---

*For specific implementation details, see the [Training Load Guide](training-load.md) and [CLI Reference](cli-reference.md).*