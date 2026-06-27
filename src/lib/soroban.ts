import {
  Account,
  Address,
  Contract,
  Keypair,
  nativeToScVal,
  Networks,
  scValToNative,
  SorobanRpc,
  TransactionBuilder,
} from '@stellar/stellar-sdk'

const RPC_URL = import.meta.env.VITE_SOROBAN_RPC_URL ?? 'https://soroban-testnet.stellar.org'
const NETWORK_PASSPHRASE = import.meta.env.VITE_STELLAR_NETWORK ?? Networks.TESTNET

const POOL_CONTRACT_ID = import.meta.env.VITE_POOL_CONTRACT_ID ?? ''
const CREDITLINE_CONTRACT_ID = import.meta.env.VITE_CREDITLINE_CONTRACT_ID ?? ''

let server: SorobanRpc.Server | null = null
let simulationAccount: Account | null = null

function getServer(): SorobanRpc.Server {
  if (!server) {
    server = new SorobanRpc.Server(RPC_URL, { allowHttp: true })
  }
  return server
}

function getSimulationSource(): Account {
  if (!simulationAccount) {
    const kp = Keypair.random()
    simulationAccount = new Account(kp.publicKey(), '0')
  }
  return simulationAccount
}

export async function checkConnection(): Promise<boolean> {
  try {
    const s = getServer()
    await s.getHealth()
    return true
  } catch {
    return false
  }
}

async function simulateContract<T>(
  contractId: string,
  method: string,
  args: any[] = [],
): Promise<T> {
  const s = getServer()
  const contract = new Contract(contractId)
  const scArgs = args.map((a) => {
    if (typeof a === 'number') return nativeToScVal(a, { type: 'u64' })
    if (typeof a === 'string') return new Address(a).toScVal()
    return nativeToScVal(a)
  })

  const tx = new TransactionBuilder(getSimulationSource(), {
    fee: '100',
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(contract.call(method, ...scArgs))
    .setTimeout(30)
    .build()

  const result = await s.simulateTransaction(tx)

  if (!SorobanRpc.Api.isSimulationSuccess(result)) {
    throw new Error(`Simulation failed for ${method}`)
  }

  return scValToNative(result.result!.retval) as T
}

export async function fetchPoolStats(): Promise<{
  totalLiquidity: string
  lockedLiquidity: string
  availableLiquidity: string
  totalShares: string
  sharePrice: string
}> {
  const native = await simulateContract<any>(POOL_CONTRACT_ID, 'get_pool_stats')
  return {
    totalLiquidity: native.total_liquidity?.toString() ?? '0',
    lockedLiquidity: native.locked_liquidity?.toString() ?? '0',
    availableLiquidity: native.available_liquidity?.toString() ?? '0',
    totalShares: native.total_shares?.toString() ?? '0',
    sharePrice: native.share_price?.toString() ?? '0',
  }
}

export async function fetchLoans(): Promise<
  Array<{
    loanId: number
    borrower: string
    vendor: string
    totalAmount: string
    guaranteeAmount: string
    interestRateBps: number
    interestAmount: string
    serviceFeeAmount: string
    principalOutstanding: string
    interestOutstanding: string
    serviceFeeOutstanding: string
    remainingBalance: string
    repaymentSchedule: Array<{
      dueDate: number
      amount: string
      paid: boolean
      paidAt: number
    }>
    status: string
    createdAt: number
    fundedAt: number
    lateFeesOutstanding: string
  }>
> {
  const count = await simulateContract<number>(CREDITLINE_CONTRACT_ID, 'get_loan_counter')
  const loans: Array<any> = []

  for (let i = 1; i <= count; i++) {
    try {
      const native = await simulateContract<any>(CREDITLINE_CONTRACT_ID, 'get_loan', [i])
      loans.push({
        loanId: Number(native.loan_id ?? i),
        borrower: native.borrower?.toString() ?? '',
        vendor: native.vendor?.toString() ?? '',
        totalAmount: native.total_amount?.toString() ?? '0',
        guaranteeAmount: native.guarantee_amount?.toString() ?? '0',
        interestRateBps: Number(native.interest_rate_bps ?? 0),
        interestAmount: native.interest_amount?.toString() ?? '0',
        serviceFeeAmount: native.service_fee_amount?.toString() ?? '0',
        principalOutstanding: native.principal_outstanding?.toString() ?? '0',
        interestOutstanding: native.interest_outstanding?.toString() ?? '0',
        serviceFeeOutstanding: native.service_fee_outstanding?.toString() ?? '0',
        remainingBalance: native.remaining_balance?.toString() ?? '0',
        repaymentSchedule: (native.repayment_schedule ?? []).map((inst: any) => ({
          dueDate: Number(inst.due_date ?? 0),
          amount: inst.amount?.toString() ?? '0',
          paid: Boolean(inst.paid),
          paidAt: Number(inst.paid_at ?? 0),
        })),
        status: native.status?.toString() ?? 'Pending',
        createdAt: Number(native.created_at ?? 0),
        fundedAt: Number(native.funded_at ?? 0),
        lateFeesOutstanding: native.late_fees_outstanding?.toString() ?? '0',
      })
    } catch {
      // skip loans that fail to load
    }
  }

  return loans
}
