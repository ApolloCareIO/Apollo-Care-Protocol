# Contributing to Apollo Care Protocol

First off, thank you for considering contributing to Apollo Care Protocol! It's people like you that make decentralized healthcare a reality.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment. We're building healthcare infrastructure—professionalism and empathy are non-negotiable.

## How Can I Contribute?

### Reporting Bugs

Before creating a bug report, please check existing issues to avoid duplicates.

**When filing a bug report, include:**
- Clear, descriptive title
- Steps to reproduce the issue
- Expected vs actual behavior
- Solana cluster (localnet/devnet/mainnet)
- Anchor/Solana CLI versions
- Relevant logs or error messages

### Suggesting Features

Feature requests are welcome! Please provide:
- Clear description of the feature
- Use case and benefits
- Any actuarial or regulatory considerations
- Potential implementation approach

### Pull Requests

1. **Fork & Clone**
   ```bash
   git clone https://github.com/YOUR_USERNAME/apollo-care-protocol.git
   cd apollo-care-protocol
   ```

2. **Create a Branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make Changes**
   - Follow the existing code style
   - Add tests for new functionality
   - Update documentation as needed

4. **Test**
   ```bash
   anchor build
   anchor test
   ```

5. **Commit**
   ```bash
   git commit -m "feat: add your feature description"
   ```
   
   We follow [Conventional Commits](https://www.conventionalcommits.org/):
   - `feat:` New feature
   - `fix:` Bug fix
   - `docs:` Documentation only
   - `test:` Adding tests
   - `refactor:` Code refactoring
   - `chore:` Maintenance tasks

6. **Push & PR**
   ```bash
   git push origin feature/your-feature-name
   ```
   Then open a Pull Request on GitHub.

## Development Guidelines

### Rust/Anchor Style

- Use `rustfmt` for formatting
- Follow Anchor best practices
- Document all public functions
- Use descriptive variable names
- Prefer safety over cleverness

### Testing Requirements

- All new features must include tests
- Maintain or improve code coverage
- Include both unit and integration tests
- Test edge cases and error conditions

### Security Considerations

Given the financial nature of this protocol:
- Never commit private keys or secrets
- Consider reentrancy in all state changes
- Use safe math operations
- Document any security assumptions
- Flag potential attack vectors in PR description

### Actuarial Accuracy

Healthcare pricing requires precision:
- All calculations must use basis points (1 bps = 0.01%)
- Document actuarial assumptions
- Reference CMS guidelines where applicable
- Consider regulatory implications

## Review Process

1. **Automated Checks**: CI must pass (build, tests, linting)
2. **Code Review**: At least one maintainer approval required
3. **Actuarial Review**: For pricing/risk changes, actuarial sign-off needed
4. **Security Review**: For critical paths, security audit may be required

## Questions?

- Open a [Discussion](https://github.com/ApolloCareIO/apollo-care-protocol/discussions)
- Tweet us [@apollocareio](https://x.com/apollocareio)
- Check existing documentation

## Recognition

Contributors will be recognized in:
- README.md contributors section
- Release notes
- Community announcements

Thank you for helping build the future of healthcare! 🏥
