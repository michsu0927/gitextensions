// Human readable approximation of the git commands behind each backend command.
// Shown in the console drawer until the real command lines arrive via the
// `git-command-log` event.

type Args = Record<string, unknown>;

const str = (v: unknown, fallback = ''): string => (typeof v === 'string' && v ? v : fallback);

const quoted = (v: unknown): string =>
  Array.isArray(v) ? v.map((f) => `"${String(f)}"`).join(' ') : '';

export function getRawGitCommand(cmd: string, args: Args): string[] {
  switch (cmd) {
    case 'get_git_version':
      return ['git --version'];
    case 'set_git_path':
      return ['git --version (validate selected executable)'];
    case 'verify_git_repo':
      return ['git rev-parse --is-inside-work-tree' + (args.repoPath ? ` (dir: ${String(args.repoPath)})` : '')];
    case 'get_git_status':
      return ['git status --porcelain=v1 -z -b', 'git status'];
    case 'get_commit_log':
      return ['git log --graph -n ' + String(args.limit ?? 100)];
    case 'get_git_branches':
      return ['git branch --format="%(refname:short)"'];
    case 'get_git_tags':
      return ['git tag'];
    case 'get_git_stashes':
      return ['git stash list'];
    case 'get_git_remotes':
      return ['git remote'];
    case 'get_git_remote_branches':
      return ['git branch -r --format="%(refname:short)"'];
    case 'configure_and_fetch_remote':
      return [
        `git remote add/set-url ${str(args.remoteName, 'origin')} ${str(args.remoteUrl)}`,
        `git fetch ${str(args.remoteName, 'origin')}`,
      ];
    case 'checkout_branch':
      return [`git checkout ${str(args.branchName)} --`];
    case 'checkout_remote_branch':
      return [`git checkout -b ${str(args.localBranch)} --track ${str(args.remoteBranch)}`];
    case 'merge_branch':
      return [`git merge --no-edit ${str(args.branchName)}`];
    case 'rebase_branch':
      return [`git rebase ${str(args.branchName)}`];
    case 'create_branch':
      return [`git branch ${str(args.newBranchName)}`];
    case 'reset_current_branch':
      return [`git reset --${str(args.mode, 'mixed')} ${str(args.target, 'HEAD')} --`];
    case 'rename_branch':
      return [`git branch -m ${str(args.oldName)} ${str(args.newName)}`];
    case 'delete_branch':
      return [`git branch ${args.force ? '-D' : '-d'} ${str(args.branchName)}`];
    case 'pull_changes':
      return ['git pull'];
    case 'push_changes':
      return ['git push'];
    case 'stage_files':
      return [`git add -- ${quoted(args.filePaths)}`];
    case 'unstage_files':
      return [`git reset -q HEAD -- ${quoted(args.filePaths)}`];
    case 'commit_changes':
      return ['git commit -F -'];
    case 'apply_stash':
      return [`git stash apply ${args.stashIndex !== undefined ? `stash@{${String(args.stashIndex)}}` : ''}`];
    case 'get_commit_files':
      return [`git diff-tree --root --no-commit-id --name-status -r -z ${str(args.commitHash)}`];
    case 'get_commit_file_diff':
      return [`git show ${str(args.commitHash)} -- "${str(args.filePath)}"`];
    case 'get_working_dir_files':
      return ['git status --porcelain=v1 -z'];
    case 'get_working_file_diff':
      return [`git diff ${args.isStaged ? '--cached ' : ''}-- "${str(args.filePath)}"`];
    case 'list_directory':
      return [`[Tauri FS] List files in ${str(args.relativeDir, '.')}`];
    case 'select_directory':
      return ['[Tauri Dialog] Select directory'];
    case 'select_git_executable':
      return ['[Tauri Dialog] Select git executable'];
    default:
      return [];
  }
}
