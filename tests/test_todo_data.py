import hashlib
import importlib.machinery
import importlib.util
import io
import json
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / 'bin/todo-data'


def load():
    loader = importlib.machinery.SourceFileLoader('todo_data', str(SCRIPT))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return module


FILES = {
    'a1b2.md': '---\ntitle: 架空の手続き\n---\nメモ\n',
    'hand_made.md': '---\ntitle: other\n---\n',
}
R2 = {'R2_ENDPOINT': 'https://account.example', 'R2_BUCKET': 'bucket',
      'R2_ACCESS_KEY_ID': 'key-id', 'R2_SECRET_ACCESS_KEY': 'dummy-secret'}


def tampered(path, entries):
    with tarfile.open(path, 'w:gz') as tar:
        for name, data in entries.items():
            entry = tarfile.TarInfo(name)
            entry.size = len(data)
            tar.addfile(entry, io.BytesIO(data))


class DataTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.data = load()

    def test_archive_restores_every_file_verbatim(self):
        archive = self.data.archive(FILES, self.root / 'backups/todos-1.tar.gz')
        with tarfile.open(archive) as tar:
            self.assertEqual(sorted(tar.getnames()), ['a1b2.md', 'hand_made.md', 'manifest.json'])
            self.assertEqual({m.mode for m in tar.getmembers()}, {0o600})
        self.data.restore(archive, self.root / 'restored')
        for name, text in FILES.items():
            self.assertEqual((self.root / 'restored' / name).read_text(), text)
        self.assertEqual(sorted(p.name for p in (self.root / 'restored').iterdir()), sorted(FILES))

    def test_an_empty_list_is_a_valid_generation(self):
        archive = self.data.archive({}, self.root / 'empty.tar.gz')
        self.data.restore(archive, self.root / 'restored')
        self.assertEqual(list((self.root / 'restored').iterdir()), [])

    def test_archive_never_overwrites_or_accepts_unsafe_names(self):
        archive = self.data.archive(FILES, self.root / 'a.tar.gz')
        before = archive.read_bytes()
        with self.assertRaises(FileExistsError):
            self.data.archive({}, archive)
        self.assertEqual(archive.read_bytes(), before)
        for name in ['../x.md', 'a/b.md', 'x.txt', '.hidden.md', 'manifest.json']:
            with self.assertRaises(ValueError, msg=name):
                self.data.archive({name: ''}, self.root / f'{len(name)}.tar.gz')

    def test_restore_rejects_tampered_archives_without_publishing(self):
        good = b'---\ntitle: t\n---\n'
        digest = hashlib.sha256(good).hexdigest()
        cases = {
            'checksum': {'a.md': b'changed', 'manifest.json': json.dumps({'a.md': digest}).encode()},
            'traversal': {'../a.md': good, 'manifest.json': json.dumps({'../a.md': digest}).encode()},
            'unlisted': {'a.md': good, 'b.md': good, 'manifest.json': json.dumps({'a.md': digest}).encode()},
            'no-manifest': {'a.md': good},
        }
        for label, entries in cases.items():
            path = self.root / f'{label}.tar.gz'
            tampered(path, entries)
            with self.assertRaises(ValueError, msg=label):
                self.data.restore(path, self.root / label)
            self.assertFalse((self.root / label).exists(), label)
        self.assertEqual(sorted(p.name for p in self.root.iterdir()),
                         sorted(f'{label}.tar.gz' for label in cases))

    def test_restore_refuses_an_existing_destination(self):
        archive = self.data.archive(FILES, self.root / 'a.tar.gz')
        (self.root / 'live').mkdir()
        (self.root / 'live' / 'keep.md').write_text('live data')
        with self.assertRaises(FileExistsError):
            self.data.restore(archive, self.root / 'live')
        self.assertEqual((self.root / 'live' / 'keep.md').read_text(), 'live data')

    def test_upload_without_credentials_fails_before_calling_the_cli(self):
        archive = self.data.archive({}, self.root / 'a.tar.gz')
        with patch.dict(os.environ, {'R2_BUCKET': 'bucket'}, clear=True), \
                patch.object(self.data.subprocess, 'run') as run:
            with self.assertRaises(ValueError) as raised:
                self.data.upload(archive)
        run.assert_not_called()
        self.assertIn('R2_ENDPOINT', str(raised.exception))
        self.assertIn('R2_SECRET_ACCESS_KEY', str(raised.exception))
        self.assertNotIn('R2_BUCKET', str(raised.exception))

    def test_upload_sends_the_archive_under_the_prefix(self):
        archive = self.data.archive({}, self.root / 'todos-1.tar.gz')
        with patch.dict(os.environ, dict(R2, R2_PREFIX='/household/todos/'), clear=True), \
                patch.object(self.data.shutil, 'which', return_value='/usr/bin/aws'), \
                patch.object(self.data.subprocess, 'run') as run:
            key = self.data.upload(archive)
        self.assertEqual(key, 'household/todos/todos-1.tar.gz')
        command, env = run.call_args.args[0], run.call_args.kwargs['env']
        self.assertIn('s3://bucket/household/todos/todos-1.tar.gz', command)
        self.assertEqual(command[command.index('--endpoint-url') + 1], R2['R2_ENDPOINT'])
        self.assertEqual(env['AWS_SECRET_ACCESS_KEY'], 'dummy-secret')
        self.assertEqual(env['AWS_DEFAULT_REGION'], 'auto')
        with patch.dict(os.environ, R2, clear=True), \
                patch.object(self.data.shutil, 'which', return_value='/usr/bin/aws'), \
                patch.object(self.data.subprocess, 'run'):
            self.assertEqual(self.data.upload(archive), 'todo-list/todos-1.tar.gz')


if __name__ == '__main__':
    unittest.main()
