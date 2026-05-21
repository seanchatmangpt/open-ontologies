# GitHub Branch Protection







resource "github_branch_protection" "open_ontologies_main" {
  repository_id = github_repository.open_ontologies.node_id
  pattern       = "main"

  require_signed_commits = true
  enforce_admins         = true

  required_pull_request_reviews {
    required_approving_review_count = 1
  }
}

