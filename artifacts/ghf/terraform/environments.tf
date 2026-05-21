# GitHub Environments




resource "github_repository_environment" "open_ontologies_audit" {
  repository  = github_repository.open_ontologies.name
  environment = "audit"
}

resource "github_repository_environment" "open_ontologies_prod" {
  repository  = github_repository.open_ontologies.name
  environment = "prod"
}

