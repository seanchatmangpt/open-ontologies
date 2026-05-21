# GitHub Labels






resource "github_issue_label" "open_ontologies_refactor" {
  repository = github_repository.open_ontologies.name
  name       = "refactor"
  color      = "f29513"
  description = "Code refactoring"
}

resource "github_issue_label" "open_ontologies_performance" {
  repository = github_repository.open_ontologies.name
  name       = "performance"
  color      = "006b75"
  description = "Performance improvements"
}

resource "github_issue_label" "open_ontologies_testing" {
  repository = github_repository.open_ontologies.name
  name       = "testing"
  color      = "fbca04"
  description = "Test coverage and improvements"
}

resource "github_issue_label" "open_ontologies_infrastructure" {
  repository = github_repository.open_ontologies.name
  name       = "infrastructure"
  color      = "5319e7"
  description = "Infrastructure or CI/CD"
}

resource "github_issue_label" "open_ontologies_documentation" {
  repository = github_repository.open_ontologies.name
  name       = "documentation"
  color      = "0075ca"
  description = "Improvements or additions to documentation"
}

resource "github_issue_label" "open_ontologies_security" {
  repository = github_repository.open_ontologies.name
  name       = "security"
  color      = "e99695"
  description = "Security related issue"
}

resource "github_issue_label" "open_ontologies_enhancement" {
  repository = github_repository.open_ontologies.name
  name       = "enhancement"
  color      = "a2eeef"
  description = "New feature or request"
}

resource "github_issue_label" "open_ontologies_bug" {
  repository = github_repository.open_ontologies.name
  name       = "bug"
  color      = "d73a4a"
  description = "Something isn't working"
}

