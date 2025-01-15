# Contributing to Fariba DDNS

Thank you for your interest in contributing to Fariba DDNS! This document provides guidelines and instructions for contributing.

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Please read it before contributing.

## Getting Started

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/your-username/fariba-ddns
   cd fariba-ddns
   ```
3. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/original-owner/fariba-ddns
   ```

## Development Process

1. Create a new branch for your feature:
   ```bash
   git checkout -b feature-name
   ```

2. Make your changes:
   - Follow the [Rust style guide](https://doc.rust-lang.org/1.0.0/style/README.html)
   - Add tests for new functionality
   - Update documentation as needed

3. Run tests:
   ```bash
   cargo test
   cargo clippy
   cargo fmt --all -- --check
   ```

4. Commit your changes:
   ```bash
   git add .
   git commit -m "Description of changes"
   ```

5. Push to your fork:
   ```bash
   git push origin feature-name
   ```

6. Create a Pull Request

## Pull Request Guidelines

1. **Title**: Use a clear, descriptive title
2. **Description**: Include:
   - What changes were made
   - Why the changes were made
   - Any related issues
3. **Changes**:
   - Keep changes focused and atomic
   - Include tests
   - Update documentation
   - Follow code style guidelines

## Code Style

- Run `cargo fmt` before committing
- Follow Rust naming conventions
- Document public APIs
- Write clear commit messages

## Documentation

- Update README.md if needed
- Add rustdoc comments for public APIs
- Update architecture.md for significant changes
- Keep docs/user-guide.md current

## Testing

- Write unit tests for new functionality
- Include integration tests where appropriate
- Test error conditions
- Verify documentation examples

## Commit Messages

Format:
```
<type>: <description>

[optional body]
[optional footer]
```

Types:
- feat: New feature
- fix: Bug fix
- docs: Documentation changes
- style: Formatting changes
- refactor: Code restructuring
- test: Test changes
- chore: Maintenance tasks

Example:
```
feat: Add support for new DNS provider

- Implement provider trait
- Add configuration options
- Write integration tests
- Update documentation

Closes #123
```

## Release Process

1. Update version in Cargo.toml
2. Update CHANGELOG.md
3. Create release PR
4. After merge, tag release
5. Create GitHub release

## Getting Help

- Open an issue for questions
- Join our community chat
- Check existing documentation

## License

By contributing, you agree that your contributions will be licensed under the GNU Affero General Public License v3.0. 