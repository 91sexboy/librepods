#!/usr/bin/env python3
"""Static regression tests for security-sensitive GitHub workflows."""

from pathlib import Path
import re
import unittest

import yaml


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
NIX_UPDATE_WORKFLOW = (
    REPOSITORY_ROOT / ".github" / "workflows" / "ci-nix-flake-update.yml"
)
LINUX_WORKFLOW = REPOSITORY_ROOT / ".github" / "workflows" / "ci-linux-rust.yml"

CHECKOUT_ACTION = (
    "actions/checkout@11d5960a326750d5838078e36cf38b85af677262"  # v4
)
NIX_INSTALLER_ACTION = (
    "DeterminateSystems/nix-installer-action@"
    "3138316df39ed29be04236d7ffc686fa525866aa"  # v23
)
UPLOAD_ARTIFACT_ACTION = (
    "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02"  # v4
)
DOWNLOAD_ARTIFACT_ACTION = (
    "actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093"  # v4
)
CREATE_PULL_REQUEST_ACTION = (
    "peter-evans/create-pull-request@"
    "5f6978faf089d4d20b00c7766989d076bb2fc7f1"  # v8.1.1
)
IMMUTABLE_ACTION_REFERENCE = re.compile(
    r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+@[0-9a-f]{40}$"
)


def load_yaml(path: Path):
    """Load YAML without YAML 1.1 coercing the GitHub `on` key."""
    return yaml.load(path.read_text(encoding="utf-8"), Loader=yaml.BaseLoader)


def named_steps(job):
    """Index a workflow job's named steps."""
    return {step["name"]: step for step in job["steps"] if "name" in step}


class WorkflowTests(unittest.TestCase):
    def test_nix_updater_is_registered_on_schedule_and_dispatch(self):
        self.assertTrue(
            NIX_UPDATE_WORKFLOW.exists(),
            "the default branch must register the scheduled Nix updater",
        )
        workflow = load_yaml(NIX_UPDATE_WORKFLOW)

        self.assertEqual([{"cron": "0 4 * * 1"}], workflow["on"]["schedule"])
        self.assertIn("workflow_dispatch", workflow["on"])

    def test_nix_updater_uses_reviewed_immutable_action_commits(self):
        workflow = load_yaml(NIX_UPDATE_WORKFLOW)
        action_references = [
            step["uses"]
            for job in workflow["jobs"].values()
            for step in job["steps"]
            if "uses" in step
        ]

        self.assertEqual(
            [
                CHECKOUT_ACTION,
                NIX_INSTALLER_ACTION,
                UPLOAD_ARTIFACT_ACTION,
                CHECKOUT_ACTION,
                DOWNLOAD_ARTIFACT_ACTION,
                CREATE_PULL_REQUEST_ACTION,
            ],
            action_references,
        )
        for action_reference in action_references:
            with self.subTest(action=action_reference):
                self.assertRegex(action_reference, IMMUTABLE_ACTION_REFERENCE)

    def test_linux_ci_nix_installer_uses_reviewed_immutable_commit(self):
        workflow = load_yaml(LINUX_WORKFLOW)
        steps = named_steps(workflow["jobs"]["build"])

        self.assertEqual(NIX_INSTALLER_ACTION, steps["Install Nix"]["uses"])
        self.assertRegex(
            steps["Install Nix"]["uses"],
            IMMUTABLE_ACTION_REFERENCE,
        )

    def test_nix_updater_checkouts_do_not_persist_credentials(self):
        workflow = load_yaml(NIX_UPDATE_WORKFLOW)
        checkout_steps = [
            step
            for job in workflow["jobs"].values()
            for step in job["steps"]
            if step.get("uses", "").startswith("actions/checkout@")
        ]

        self.assertEqual(2, len(checkout_steps))
        for checkout in checkout_steps:
            with self.subTest(job_checkout=checkout["name"]):
                self.assertEqual(
                    "false",
                    checkout.get("with", {}).get("persist-credentials"),
                )

    def test_nix_updater_isolates_validation_from_pull_request_writes(self):
        workflow = load_yaml(NIX_UPDATE_WORKFLOW)
        self.assertIn("create-pull-request", workflow["jobs"])
        update_job = workflow["jobs"]["update-lock"]
        pull_request_job = workflow["jobs"]["create-pull-request"]

        self.assertEqual({}, workflow["permissions"])
        self.assertEqual({"contents": "read"}, update_job["permissions"])
        self.assertEqual(
            {"contents": "write", "pull-requests": "write"},
            pull_request_job["permissions"],
        )
        self.assertEqual("update-lock", pull_request_job["needs"])
        self.assertEqual(
            "${{ steps.lockfile.outputs.changed }}",
            update_job["outputs"]["changed"],
        )
        self.assertEqual(
            "needs.update-lock.outputs.changed == 'true'",
            pull_request_job["if"],
        )
        self.assertFalse(
            any(
                step.get("uses", "").startswith(
                    "peter-evans/create-pull-request@"
                )
                for step in update_job["steps"]
            )
        )
        self.assertEqual(
            [
                "Checkout target branch",
                "Download updated lockfile",
                "Open flake lock update pull request",
            ],
            [step["name"] for step in pull_request_job["steps"]],
        )
        self.assertFalse(any("run" in step for step in pull_request_job["steps"]))

    def test_nix_updater_targets_linux_rust(self):
        workflow = load_yaml(NIX_UPDATE_WORKFLOW)
        self.assertIn("create-pull-request", workflow["jobs"])
        update_steps = named_steps(workflow["jobs"]["update-lock"])
        pull_request_steps = named_steps(workflow["jobs"]["create-pull-request"])

        self.assertEqual(
            "linux/rust",
            update_steps["Checkout target branch"]["with"]["ref"],
        )
        self.assertEqual(
            "linux/rust",
            pull_request_steps["Checkout target branch"]["with"]["ref"],
        )
        self.assertEqual(
            "linux/rust",
            pull_request_steps["Open flake lock update pull request"]["with"][
                "base"
            ],
        )

    def test_nix_updater_hands_off_only_flake_lock(self):
        workflow = load_yaml(NIX_UPDATE_WORKFLOW)
        self.assertIn("create-pull-request", workflow["jobs"])
        update_job = workflow["jobs"]["update-lock"]
        pull_request_job = workflow["jobs"]["create-pull-request"]
        update_steps = named_steps(update_job)
        pull_request_steps = named_steps(pull_request_job)
        self.assertIn("Upload updated lockfile", update_steps)
        self.assertIn("Download updated lockfile", pull_request_steps)
        upload = update_steps["Upload updated lockfile"]
        download = pull_request_steps["Download updated lockfile"]
        create_pull_request = pull_request_steps[
            "Open flake lock update pull request"
        ]

        self.assertEqual(
            "steps.lockfile.outputs.changed == 'true'",
            upload["if"],
        )
        self.assertEqual("flake-lock-update", upload["with"]["name"])
        self.assertEqual("flake.lock", upload["with"]["path"])
        self.assertEqual("error", upload["with"]["if-no-files-found"])
        self.assertEqual(upload["with"]["name"], download["with"]["name"])
        self.assertEqual(".", download["with"]["path"])
        self.assertEqual("flake.lock", create_pull_request["with"]["add-paths"])


if __name__ == "__main__":
    unittest.main()
