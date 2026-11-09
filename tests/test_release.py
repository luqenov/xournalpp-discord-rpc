import argparse
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "release", Path(__file__).resolve().parents[1] / "scripts/release.py"
)
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.tag = "v0.2.0"
        self.directory = self.root / "dist" / self.tag
        self.directory.mkdir(parents=True)
        self.archive = self.directory / f"xournal-rpc-{self.tag}-linux-x86_64.tar.gz"
        self.archive.write_bytes(b"test archive")
        (self.directory / "SHA256SUMS.txt").write_text(release.checksums([self.archive]))
        self.args = argparse.Namespace(tag=self.tag, notes=None, dry_run=False)
        self.root_patch = patch.object(release, "ROOT", self.root)
        self.root_patch.start()
        self.addCleanup(self.root_patch.stop)

    def test_rejects_modified_archives(self):
        self.archive.write_bytes(b"corrupted")
        with self.assertRaisesRegex(ValueError, "does not match"):
            release.release_files(self.directory, self.tag)

    @patch.object(release, "validate_tag")
    def test_build_packages_both_architectures(self, validate):
        # Build artifacts are mocked; packaging and checksums run for real.
        import tarfile
        import shutil
        shutil.rmtree(self.directory)
        for target in release.TARGETS:
            binary = self.root / "target" / target / "release/xournal-rpc"
            binary.parent.mkdir(parents=True)
            binary.write_bytes(target.encode())
            binary.chmod(0o755)
        for name in ["README.md", "LICENSE", "packaging/xournal-rpc.service"]:
            path = self.root / name
            path.parent.mkdir(exist_ok=True)
            path.write_text(name)
        args = argparse.Namespace(tag=self.tag, target=None, builder="zigbuild")
        with patch.object(release.subprocess, "run") as run, \
                patch.dict("os.environ", {"CARGO_TARGET_DIR": "target"}):
            release.build(args)
        self.assertEqual(run.call_count, 2)
        files = release.release_files(self.directory, self.tag)
        self.assertEqual(len(files), 3)
        for archive in files[:-1]:
            with tarfile.open(archive) as package:
                self.assertEqual(set(package.getnames()), {
                    "xournal-rpc", "README.md", "LICENSE", "xournal-rpc.service",
                })
                self.assertTrue(package.getmember("xournal-rpc").mode & 0o111)

    def test_draft_with_unexpected_assets_is_not_published(self):
        with self.assertRaisesRegex(ValueError, "unexpected assets"):
            self.publish_with([
                (200, {"commit": {"sha": "commit"}}),
                (200, {"id": 42, "draft": True,
                       "assets": [{"id": 7, "name": "old-build.tar.gz"}]}),
            ])

    def test_tag_must_match_manifest_and_checkout(self):
        (self.root / "Cargo.toml").write_text('[package]\nversion = "0.2.0"\n')
        with self.assertRaisesRegex(ValueError, "Cargo.toml"):
            release.validate_tag("v9.9.9")
        with patch.object(release, "git", return_value="modified"):
            with self.assertRaisesRegex(ValueError, "commit or stash"):
                release.validate_tag(self.tag)
        with patch.object(release, "git", side_effect=["", "tag-commit", "head-commit"]):
            with self.assertRaisesRegex(ValueError, "check out"):
                release.validate_tag(self.tag)

    @patch.object(release, "validate_tag")
    @patch.object(release, "api")
    def test_dry_run_has_no_network_requests(self, api, validate):
        self.args.dry_run = True
        release.publish(self.args)
        api.assert_not_called()

    def publish_with(self, responses, upload_error=None):
        with patch.object(release, "validate_tag"), \
                patch.object(release, "git", return_value="commit"), \
                patch.dict("os.environ", {"CODEBERG_TOKEN": "test-token"}), \
                patch.object(release, "api", side_effect=responses) as api, \
                patch.object(release, "upload", side_effect=upload_error) as upload:
            release.publish(self.args)
            return api, upload

    def test_creates_draft_uploads_then_publishes(self):
        api, upload = self.publish_with([
            (200, {"commit": {"sha": "commit"}}), (404, None),
            (201, {"id": 42, "draft": True, "assets": []}),
            (200, {"html_url": "https://codeberg.org/release"}),
        ])
        self.assertTrue(api.call_args_list[2].args[3]["draft"])
        self.assertEqual(upload.call_count, 2)
        self.assertEqual(api.call_args_list[-1].args[0], "PATCH")
        self.assertFalse(api.call_args_list[-1].args[3]["draft"])

    def test_upload_failure_leaves_release_in_draft(self):
        with patch.object(release, "api", side_effect=[
            (200, {"commit": {"sha": "commit"}}), (404, None),
            (201, {"id": 42, "draft": True, "assets": []}),
        ]) as api, patch.object(release, "validate_tag"), \
                patch.object(release, "git", return_value="commit"), \
                patch.dict("os.environ", {"CODEBERG_TOKEN": "test-token"}), \
                patch.object(release, "upload", side_effect=RuntimeError("upload failed")):
            with self.assertRaisesRegex(RuntimeError, "upload failed"):
                release.publish(self.args)
        self.assertFalse(any(call.args[0] in ("PATCH", "DELETE")
                             for call in api.call_args_list))

    def test_published_release_cannot_be_overwritten(self):
        with self.assertRaisesRegex(ValueError, "already published"):
            self.publish_with([
                (200, {"commit": {"sha": "commit"}}),
                (200, {"id": 42, "draft": False}),
            ])

    def test_remote_tag_must_match_local_commit(self):
        with self.assertRaisesRegex(ValueError, "local release commit"):
            self.publish_with([(200, {"commit": {"sha": "different"}})])

    def test_retry_reuses_draft_and_replaces_only_matching_asset(self):
        api, _ = self.publish_with([
            (200, {"commit": {"sha": "commit"}}),
            (200, {"id": 42, "draft": True,
                   "assets": [{"id": 7, "name": self.archive.name}]}),
            (204, None), (200, {"html_url": "https://codeberg.org/release"}),
        ])
        self.assertEqual(api.call_args_list[2].args[:2],
                         ("DELETE", "/releases/42/assets/7"))
        self.assertFalse(any(call.args[:2] == ("DELETE", "/releases/42")
                             for call in api.call_args_list))

    @patch.object(release, "api")
    def test_upload_uses_multipart_attachment(self, api):
        release.upload(42, self.archive, "test-token")
        args = api.call_args.args
        self.assertIn(b'name="attachment"', args[3])
        self.assertIn(b"test archive", args[3])
        self.assertTrue(args[4].startswith("multipart/form-data; boundary="))

    def test_api_spaces_requests(self):
        with patch.object(release, "_next_request_at", 0.0), \
                patch.object(release.time, "monotonic", side_effect=[100.0, 100.25]), \
                patch.object(release.time, "sleep") as sleep, \
                patch.object(release.urllib.request, "urlopen") as urlopen:
            response = urlopen.return_value.__enter__.return_value
            response.status = 200
            response.read.return_value = b'{}'
            release.api("GET", "/tags/v0.2.0", "test-token")
            release.api("GET", "/releases/tags/v0.2.0", "test-token")
            sleep.assert_called_once_with(0.75)
            self.assertEqual(urlopen.call_count, 2)

    def test_rate_limit_stops_without_retry(self):
        error = release.urllib.error.HTTPError(
            "https://codeberg.org", 429, "Too Many Requests", {"Retry-After": "60"}, None
        )
        with patch.object(release, "_next_request_at", 0.0), \
                patch.object(release.urllib.request, "urlopen", side_effect=error) as urlopen:
            with self.assertRaisesRegex(RuntimeError, "Retry-After: 60"):
                release.api("GET", "/tags/v0.2.0", "test-token")
            urlopen.assert_called_once()


if __name__ == "__main__":
    unittest.main()
