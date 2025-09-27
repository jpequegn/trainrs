# Contributing to TrainRS

Welcome to TrainRS! We're excited that you want to contribute to this open-source training analysis platform. This guide will help you understand how to contribute effectively, whether you're a **sports scientist** wanting to improve the methodologies or a **developer** looking to enhance the codebase.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [For Sports Scientists](#for-sports-scientists)
- [For Developers](#for-developers)
- [Development Setup](#development-setup)
- [Testing Guidelines](#testing-guidelines)
- [Documentation Standards](#documentation-standards)
- [Submission Process](#submission-process)
- [Community & Support](#community--support)

## Code of Conduct

This project follows a respectful, inclusive community standard:

- **Respectful**: Treat all contributors with respect, regardless of experience level
- **Collaborative**: Work together to solve problems and improve the project
- **Evidence-Based**: Support suggestions with research, data, or practical experience
- **Professional**: Maintain professional communication in all interactions

## Getting Started

### Prerequisites

- **Rust 1.70+**: Required for building TrainRS
- **Git**: For version control and contributing
- **Understanding**: Basic familiarity with training science or software development
- **GitHub Account**: For submitting contributions

### First Steps

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/yourusername/trainrs.git
   cd trainrs
   ```
3. **Set up the development environment** (see [Development Setup](#development-setup))
4. **Read the documentation** in the `docs/` directory
5. **Look for issues** labeled `good-first-issue` or `help-wanted`

## For Sports Scientists

Your expertise in training methodology, exercise physiology, and sports science is invaluable to TrainRS. Here's how you can contribute:

### Areas of Contribution

#### 1. **Methodology Validation & Improvement**
- **Algorithm Accuracy**: Validate TSS, PMC, and other calculations against research
- **Zone Models**: Propose improvements to power, heart rate, and pace zone calculations
- **Sport-Specific Adaptations**: Suggest scaling factors and adaptations for different sports
- **Research Integration**: Propose new metrics based on recent sports science research

#### 2. **Documentation Enhancement**
- **Sports Science Guide**: Improve the theoretical explanations in `docs/sports-science.md`
- **Training Load Guide**: Enhance interpretation guidelines in `docs/training-load.md`
- **FAQ Updates**: Add common questions from coaches and athletes
- **Case Studies**: Provide real-world examples of TrainRS application

#### 3. **Feature Requests**
- **New Metrics**: Propose additional training load or performance metrics
- **Periodization Models**: Suggest new training periodization approaches
- **Multi-Sport Features**: Improve support for triathlon, duathlon, and other multi-sport training
- **Analysis Tools**: Request new analysis or visualization capabilities

### Contribution Process for Sports Scientists

#### Research-Based Contributions
1. **Literature Review**: Cite peer-reviewed research supporting your proposal
2. **Methodology**: Explain the scientific basis and calculation method
3. **Validation**: Provide test cases or expected outcomes
4. **Implementation Guidance**: Work with developers to implement correctly

#### Documentation Contributions
1. **Accuracy**: Ensure all scientific statements are accurate and cited
2. **Clarity**: Write for both technical and non-technical audiences
3. **Practical Examples**: Include real-world applications and case studies
4. **Citations**: Use proper academic citation format

#### Example Contribution: New Training Metric

```markdown
## Proposed Feature: Training Impulse (TRIMP)

### Scientific Basis
TRIMP quantifies training load using heart rate data (Banister et al., 1991).

### Formula
TRIMP = Duration × HR_reserve × e^(k × HR_reserve)
Where k = 1.92 (men), 1.67 (women)

### Implementation Requirements
- Heart rate data validation
- Gender-specific exponential factors
- Integration with existing PMC calculations

### Expected Outcomes
- Alternative load metric for non-power sports
- Better correlation with perceived exertion
- Enhanced multi-sport analysis capabilities

### References
Banister, E. W. (1991). Modeling elite athletic performance.
```

## For Developers

Your programming skills help bring sports science concepts to life. Here's how you can contribute:

### Areas of Contribution

#### 1. **Core Features**
- **New Commands**: Implement new CLI commands and functionality
- **Data Import/Export**: Support additional file formats (FIT, TCX, Strava API)
- **Performance Optimization**: Improve calculation speed and memory usage
- **Database Enhancements**: Optimize data storage and retrieval

#### 2. **Code Quality**
- **Refactoring**: Improve code organization and maintainability
- **Error Handling**: Enhance error messages and recovery mechanisms
- **Testing**: Add unit tests, integration tests, and benchmarks
- **Documentation**: Improve code comments and API documentation

#### 3. **User Experience**
- **CLI Improvements**: Better command-line interface and help text
- **Output Formatting**: Enhanced tables, charts, and export formats
- **Configuration**: Improved configuration management and validation
- **Cross-Platform**: Ensure compatibility across operating systems

### Technical Standards

#### Code Style
- **Formatting**: Use `cargo fmt` for consistent formatting
- **Linting**: Address all `cargo clippy` warnings
- **Naming**: Use clear, descriptive names for functions and variables
- **Comments**: Document complex algorithms and sports science calculations

#### Architecture Principles
- **Modularity**: Keep sport-specific logic in separate modules
- **Error Handling**: Use `anyhow` for error propagation and `thiserror` for custom errors
- **Performance**: Use `rayon` for parallel processing when appropriate
- **Precision**: Use `rust_decimal` for exact arithmetic in calculations

#### Dependencies
- **Minimal**: Only add dependencies that provide significant value
- **Maintained**: Use well-maintained crates with active development
- **Licensing**: Ensure compatibility with MIT license
- **Security**: Regularly audit dependencies for vulnerabilities

### Code Structure

```
src/
├── lib.rs              # Library entry point
├── main.rs             # CLI entry point
├── models.rs           # Data models and structures
├── database.rs         # Database operations
├── config.rs           # Configuration management
├── tss.rs              # Training Stress Score calculations
├── pmc.rs              # Performance Management Chart
├── zones.rs            # Training zone calculations
├── power.rs            # Power analysis (cycling)
├── running.rs          # Running-specific analysis
├── multisport.rs       # Multi-sport training
├── training_plan.rs    # Training plan generation
├── performance.rs      # Performance optimization
├── data_management.rs  # Data processing utilities
├── import/             # Data import modules
│   ├── mod.rs
│   ├── csv.rs
│   ├── gpx.rs
│   ├── fit.rs
│   ├── tcx.rs
│   ├── validation.rs
│   └── streaming.rs
└── export/             # Data export modules
    ├── mod.rs
    ├── csv.rs
    ├── json.rs
    └── text.rs
```

## Development Setup

### Environment Setup

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Clone and build**:
   ```bash
   git clone https://github.com/jpequegn/trainrs.git
   cd trainrs
   cargo build
   ```

3. **Run tests**:
   ```bash
   cargo test
   cargo clippy
   cargo fmt
   ```

4. **Try the CLI**:
   ```bash
   cargo run -- --help
   ```

### Development Tools

- **IDE**: Use VS Code with rust-analyzer extension
- **Debugging**: Use `cargo run` with `--verbose` flag for detailed output
- **Profiling**: Use `cargo bench` for performance testing
- **Documentation**: Use `cargo doc --open` to view API docs

### Setting Up Test Data

1. **Create test directory**:
   ```bash
   mkdir test_data
   ```

2. **Generate sample data**:
   ```bash
   cargo run -- config --init
   # Add sample workout data for testing
   ```

## Testing Guidelines

### Test Categories

#### Unit Tests
- **Location**: Same file as the code being tested
- **Coverage**: All calculation functions must have unit tests
- **Precision**: Use exact decimal comparisons for sports science calculations

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_tss_calculation() {
        let duration_seconds = 3600; // 1 hour
        let normalized_power = dec!(200);
        let ftp = dec!(250);

        let result = calculate_tss(duration_seconds, normalized_power, ftp);
        let expected = dec!(64.0); // Expected TSS

        assert_eq!(result, expected);
    }
}
```

#### Integration Tests
- **Location**: `tests/` directory
- **Purpose**: Test CLI commands and workflows
- **Data**: Use realistic training data for testing

#### Benchmarks
- **Location**: `benches/` directory
- **Purpose**: Monitor performance of calculations
- **Standards**: TSS calculation should process 1000 workouts in <100ms

### Test Data

#### Valid Test Cases
- **Realistic Values**: Use data that reflects real training scenarios
- **Edge Cases**: Test boundary conditions (zero power, maximum values)
- **Different Sports**: Include cycling, running, swimming examples

#### Sports Science Validation
- **Known Results**: Test against manually calculated examples
- **Research Data**: Use data from published studies when available
- **Cross-Platform**: Ensure calculations match other platforms

## Documentation Standards

### Code Documentation

#### Function Documentation
```rust
/// Calculates Training Stress Score (TSS) using the Coggan formula.
///
/// TSS quantifies training stress by combining intensity and duration.
/// Based on the original algorithm by Dr. Andy Coggan.
///
/// # Arguments
/// * `duration_seconds` - Workout duration in seconds
/// * `normalized_power` - Normalized Power in watts
/// * `ftp` - Functional Threshold Power in watts
///
/// # Returns
/// TSS value as a Decimal for exact arithmetic
///
/// # Examples
/// ```
/// use rust_decimal_macros::dec;
/// let tss = calculate_tss(3600, dec!(200), dec!(250));
/// assert_eq!(tss, dec!(64.0));
/// ```
///
/// # References
/// Coggan, A. R., & Allen, H. (2006). Training and Racing with a Power Meter.
fn calculate_tss(duration_seconds: u32, normalized_power: Decimal, ftp: Decimal) -> Decimal
```

### User Documentation

#### Markdown Standards
- **Headers**: Use descriptive headers with proper hierarchy
- **Code Blocks**: Include language specification for syntax highlighting
- **Examples**: Provide realistic, working examples
- **Cross-References**: Link between related documentation sections

#### Writing Style
- **Clarity**: Write for both technical and non-technical audiences
- **Accuracy**: Ensure all technical information is correct
- **Completeness**: Cover all aspects of the feature or concept
- **Updates**: Keep documentation current with code changes

## Submission Process

### Before Submitting

1. **Read the issue** thoroughly and understand the requirements
2. **Discuss approach** in the issue comments if making significant changes
3. **Write tests** for new functionality
4. **Update documentation** for any user-facing changes
5. **Run the full test suite** and ensure all tests pass

### Pull Request Guidelines

#### Title Format
- **Feature**: `feat: add TRIMP calculation for heart rate-based training load`
- **Bug Fix**: `fix: correct TSS calculation for workouts under 20 minutes`
- **Documentation**: `docs: improve multi-sport training analysis guide`
- **Refactor**: `refactor: extract common zone calculation logic`

#### Description Template
```markdown
## Description
Brief description of the changes and their purpose.

## Type of Change
- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Sports Science Validation
For algorithm changes:
- [ ] Calculations validated against published research
- [ ] Test cases include realistic training data
- [ ] Results match expected values from manual calculation

## Testing
- [ ] New tests added for new functionality
- [ ] All existing tests pass
- [ ] Benchmarks run and performance is acceptable

## Documentation
- [ ] Code comments updated
- [ ] User documentation updated
- [ ] Sports science explanations included where relevant

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex algorithms
- [ ] Documentation updated
- [ ] Tests added and passing
```

#### Code Review Process

1. **Automated Checks**: GitHub Actions will run tests and linting
2. **Sports Science Review**: For algorithm changes, sports science validation required
3. **Code Review**: Core maintainers will review code quality and architecture
4. **Integration Testing**: Changes tested with existing functionality
5. **Documentation Review**: User-facing documentation checked for accuracy

### Commit Messages

Use conventional commit format:
- `feat(tss): add sport-specific scaling factors`
- `fix(import): handle malformed CSV files gracefully`
- `docs(zones): improve heart rate zone calculation explanation`
- `test(pmc): add integration tests for PMC calculation`

## Community & Support

### Communication Channels

- **GitHub Issues**: Bug reports, feature requests, and discussions
- **GitHub Discussions**: General questions and community help
- **Email**: For security issues or sensitive topics

### Getting Help

#### For Sports Scientists
- **Methodology Questions**: Discuss in GitHub Discussions with "sports-science" tag
- **Research Citations**: Help needed with finding appropriate research references
- **Validation**: Assistance with validating algorithm implementations

#### For Developers
- **Technical Issues**: Create GitHub issues with detailed reproduction steps
- **Architecture Questions**: Discuss design decisions in issue comments
- **Code Review**: Request feedback on significant changes before submitting

### Recognition

Contributors are recognized in several ways:
- **README Credits**: All contributors listed in the main README
- **Release Notes**: Significant contributions highlighted in release announcements
- **Documentation**: Sports science contributors credited in methodology sections

## Sports Science Research Guidelines

### Citation Standards

#### Peer-Reviewed Sources
- Use recent research (within 10 years when possible)
- Prefer sports science journals (e.g., Medicine & Science in Sports & Exercise)
- Include DOI when available

#### Citation Format
```
Author, A. A. (Year). Title of article. Journal Name, Volume(Issue), pages. DOI
```

#### Example
```
Coggan, A. R., & Allen, H. (2006). Training and Racing with a Power Meter. VeloPress.
Seiler, S. (2010). What is best practice for training intensity and duration distribution in endurance athletes? International Journal of Sports Physiology and Performance, 5(3), 276-291.
```

### Algorithm Validation

#### Validation Process
1. **Literature Review**: Find authoritative sources for the algorithm
2. **Manual Calculation**: Verify implementation with hand calculations
3. **Cross-Platform Testing**: Compare results with established platforms
4. **Expert Review**: Have sports scientists validate the implementation

#### Test Case Development
- **Known Results**: Use published examples with known outcomes
- **Edge Cases**: Test boundary conditions and unusual scenarios
- **Real Data**: Validate with actual training data from athletes

## Development Roadmap

### Current Focus Areas

- **Data Import**: FIT file support, Strava API integration
- **Analysis**: Advanced power analysis, running metrics
- **Visualization**: Chart generation, trend analysis
- **Multi-Sport**: Triathlon-specific features

### Long-Term Goals

- **Web Interface**: Browser-based dashboard
- **Mobile App**: iOS/Android applications
- **Real-Time**: Live data streaming and analysis
- **Machine Learning**: Predictive performance modeling

---

Thank you for contributing to TrainRS! Your expertise helps make training analysis more accessible and accurate for athletes, coaches, and sports scientists worldwide.

For questions about contributing, please open a GitHub Discussion or contact the maintainers through GitHub issues.