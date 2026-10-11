#!/usr/bin/env bash
# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  you may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------

set -euo pipefail

main() {
  if [[ "$#" -ne 0 ]]; then
    printf '%s\n' "Usage: ${0}" >&2
    return 2
  fi

  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
  repo_root="$(cd "${script_dir}/../../.." && pwd -P)"
  python_bin="${repo_root}/python/.venv/bin/python"

  if [[ -z "${HYPERLIQUID_TESTNET_PK:-}" ]]; then
    printf '%s\n' 'HYPERLIQUID_TESTNET_PK is required' >&2
    return 1
  fi

  if [[ ! -x "${python_bin}" ]]; then
    printf '%s\n' "Expected project Python at ${python_bin}" >&2
    return 1
  fi

  export HYPERLIQUID_OUTCOME_ACTION_QTY=1
  export HYPERLIQUID_OUTCOME_ACTION_EXECUTE=1

  printf '%s\n' 'Running one 1 USDC HIP-4 split/merge cycle on Hyperliquid testnet' >&2
  exec "${python_bin}" "${script_dir}/outcome_actions_tester.py"
}

main "$@"
