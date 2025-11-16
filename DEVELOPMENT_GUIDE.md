# PhotoMap Development Guide

## 🎯 Philosophy & Principles

### Core Principles
1. **Simplicity First** - Solve problems directly without over-engineering
2. **Minimal Dependencies** - Use standard library whenever possible
3. **Cross-Platform First** - Consider Windows/macOS/Linux from day one
4. **Single Instance** - Prevent conflicts and confusion
5. **Clean Architecture** - Each module has one clear responsibility

### Development Strategy
- **Front-load Problem Solving** - Identify edge cases early
- **Research-Driven** - Always search for 2025 best practices
- **User-Focused** - Ask clarifying questions before implementation
- **Iterative Improvement** - Small, testable increments

## 🔧 Development Workflow

### 1. Planning Phase
```
Task Identified → User Questions → Research → Implementation Plan → Code Review
```

#### User Questions Template:
- What specific problem are we solving?
- What are the edge cases to consider?
- What platforms need to be supported?
- Are there existing solutions we should reference?
- What does success look like?

#### Research Process:
1. Search for 2025 best practices
2. Cross-reference multiple sources
3. Consider standard library alternatives
4. Document findings with sources

### 2. Implementation Phase
- Create todos for each step
- Implement incrementally
- Test each change immediately
- Update documentation as you go

### 3. Code Review Checklist
- [ ] Dependencies: Are they absolutely necessary?
- [ ] Error handling: Is it comprehensive?
- [ ] Platform compatibility: Windows/macOS/Linux
- [ ] Performance: Any obvious bottlenecks?
- [ ] Documentation: Is it up to date?
- [ ] Version numbers: Updated everywhere?

## 📝 Project Structure Standards

### Module Organization
```
src/
├── main.rs              # Entry point, minimal logic
├── database.rs          # Database operations only
├── server.rs            # HTTP endpoints only
├── image_processing.rs  # Image manipulation
├── exif_parser.rs       # Metadata extraction
├── html_template.rs     # UI template generation
├── settings.rs          # Configuration management
├── process_manager.rs   # Process lifecycle
├── constants.rs         # Shared constants
└── utils/               # (optional) shared utilities
```

### File Numbering Convention
When creating multiple versions of similar functionality:
- Use descriptive names over numbers
- Keep version history in git, not filenames
- Exception: Temporary investigation files can be numbered

## 🚀 Feature Implementation Guidelines

### Before Starting
1. **Ask Questions**: Use the User Questions template
2. **Research 2025 Solutions**: Search current best practices
3. **Plan Architecture**: Consider dependencies and complexity
4. **Create Todos**: Break into small, testable steps

### During Implementation
1. **Prefer Standard Library**: Check std:: before adding dependencies
2. **Handle Errors Gracefully**: Use Result<> patterns
3. **Consider All Platforms**: Windows/macOS/Linux differences
4. **Write Tests**: If functionality is complex
5. **Update Documentation**: As you implement

### After Implementation
1. **Code Review**: Use the checklist
2. **Test Manually**: Verify functionality works
3. **Update README**: If user-facing changes
4. **Commit**: With clear, descriptive message
5. **Push**: Immediately after successful testing

## 🔍 Research Protocol

### Search Queries Template
```
"[feature_name] best practices 2025"
"[feature_name] rust standard library alternative"
"[feature_name] cross-platform implementation"
"[feature_name] performance optimization"
```

### Source Prioritization
1. Official Rust documentation
2. Well-maintained crates (if absolutely necessary)
3. Recent Stack Overflow answers (2024-2025)
4. Official platform documentation (Windows/macOS/Linux)

### Documentation Requirements
- Always cite sources for major decisions
- Note why alternatives were rejected
- Document platform-specific considerations

## 📦 Dependency Management

### Before Adding Dependencies
1. **Check Standard Library**: Is there an std:: alternative?
2. **Question Necessity**: Do we really need this?
3. **Consider Maintenance**: Is the crate well-maintained?
4. **Check Size**: How much will this add to binary?

### Preferred Dependencies
- Small, focused crates
- Actively maintained (2024+ commits)
- Minimal transitive dependencies
- Clear documentation

### Dependency Anti-Patterns
- Adding dependencies for convenience only
- Large frameworks for small tasks
- Dependencies with many features we don't need

## 🔄 Version Management

### Version Numbering
- Follow Semantic Versioning (MAJOR.MINOR.PATCH)
- Update ALL version references:
  - `Cargo.toml`
  - `main.rs` startup message
  - `html_template.rs` UI display
  - `README.md` title and version history
  - Any other locations

### Version History Format
```markdown
### v0.X.X (Current) - [Edition Name]
- ✅ **Feature Category** - Brief description of improvement
- ✅ **Another Category** - Another improvement
- ✅ **User Benefit** - What this means for users

### v0.X.X - [Previous Edition]
- Previous version notes...
```

## 🧪 Testing Strategy

### Manual Testing Checklist
- [ ] Application starts without errors
- [ ] Core functionality works
- [ ] Edge cases are handled
- [ ] Multiple instances work correctly
- [ ] Error conditions are user-friendly

### Platform Testing
- **macOS**: Development platform, tested continuously
- **Windows**: Test before major releases
- **Linux**: Test community-reported issues

### Performance Testing
- Measure startup time
- Test with realistic photo collections
- Monitor memory usage
- Check for memory leaks

## 📋 Common Tasks

### Adding New Feature
1. Research 2025 best practices
2. Ask user clarifying questions
3. Create implementation plan
4. Break into todos
5. Implement incrementally
6. Test thoroughly
7. Update documentation
8. Commit and push

### Fixing Bug
1. Reproduce issue consistently
2. Identify root cause
3. Consider edge cases
4. Implement minimal fix
5. Test fix doesn't break other functionality
6. Update documentation if needed
7. Commit with bug reference

### Performance Optimization
1. Profile current performance
2. Identify bottlenecks
3. Research optimization techniques
4. Implement changes
5. Measure improvement
6. Document performance gains

## 🎨 UI/UX Guidelines

### Web Interface Principles
- Responsive design for all screen sizes
- Clear feedback for user actions
- Graceful error handling
- Minimal chrome, maximum content
- Consistent styling

### Performance Considerations
- Lazy load images and data
- Minimize DOM manipulations
- Use efficient CSS selectors
- Test with large photo collections

## 📚 Documentation Standards

### Code Comments
- Explain complex algorithms
- Document platform-specific code
- Note why certain approaches were chosen
- Keep comments up to date with code

### README Updates
- Update version history for each release
- Document new features clearly
- Update installation instructions if needed
- Add troubleshooting for common issues

### API Documentation
- Document all public functions
- Include examples for complex usage
- Note platform-specific behavior
- Keep documentation in sync with code

## 🔒 Security Considerations

### Input Validation
- Validate all user input
- Sanitize file paths
- Check file sizes and types
- Prevent path traversal attacks

### Error Messages
- Don't expose internal system details
- Provide helpful, actionable error messages
- Log technical details for debugging
- User-friendly error presentation

## 🚀 Deployment Guidelines

### Release Process
1. Final testing on all target platforms
2. Update version numbers everywhere
3. Update documentation
4. Commit with version tag
5. Create release notes
6. Push to repository

### Binary Distribution
- Test binary on clean system
- Verify dependencies are bundled
- Check file permissions
- Test installation process

## 🔄 Continuous Improvement

### Regular Reviews
- Monthly: Review dependencies for updates
- Quarterly: Review architecture for improvements
- Annually: Evaluate technology choices

### Community Feedback
- Monitor GitHub issues
- Consider user feature requests
- Track bug reports and patterns
- Update documentation based on questions

---

## 🎯 Current Development Priorities

1. **Windows Compatibility**: Ensure full Windows support
2. **Performance Optimization**: Large photo collections (>10k photos)
3. **Advanced Features**: Photo clustering, advanced filtering
4. **Mobile Support**: Responsive design for mobile devices
5. **Internationalization**: Multi-language support

---

*This guide evolves with the project. Update it when you learn new best practices or discover better approaches.*