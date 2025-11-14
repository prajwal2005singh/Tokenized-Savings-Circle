import React, { useState } from 'react';
import { createCircle } from '../../utils/soroban';
// import { useWallet } from '../components/WalletConnect'; // Assume a context hook exists

// Mock Wallet Hook for scaffolding
const useWallet = () => ({
    userKey: "GAU2I6W4G3TWHV7L3J4J3Y2G5G7F3H3K3L5M6N8P7Q9R5S4U3V2X1Z", // Placeholder key
    isConnected: true,
});

/**
 * Creates a new Savings Circle (ROSCA) by calling the Soroban smart contract.
 * Path: frontend/src/pages/CreateCircle.tsx
 */
const CreateCirclePage: React.FC = () => {
    const { userKey, isConnected } = useWallet(); // Get current user's public key (signer)
    
    const [tokenAsset, setTokenAsset] = useState<string>('CDAVQ2S...TESTNET_TOKEN_ID');
    const [depositAmount, setDepositAmount] = useState<number>(100); // 100 units
    const [cycleIntervalHours, setCycleIntervalHours] = useState<number>(72); // 72 hours (3 days)
    const [memberKeys, setMemberKeys] = useState<string>('GB...KEY1, GC...KEY2, GD...KEY3');
    const [loading, setLoading] = useState<boolean>(false);
    const [txStatus, setTxStatus] = useState<string | null>(null);
    const [error, setError] = useState<string | null>(null);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!userKey || !isConnected) {
            setError("Please connect your wallet to create a circle.");
            return;
        }

        setLoading(true);
        setError(null);
        setTxStatus(null);

        try {
            const membersArray = memberKeys.split(',').map(k => k.trim()).filter(k => k.length > 0);
            
            // Convert hours to seconds
            const cycleIntervalSecs = cycleIntervalHours * 60 * 60;
            // Use 24 hours as a simple join deadline
            const joinDeadlineSecs = 24 * 60 * 60; 

            // Convert deposit amount to i128 friendly format (assuming 7 token decimals)
            // NOTE: Must check token decimals!
            const amountInBaseUnits = BigInt(depositAmount) * BigInt(10_000_000); 

            const result = await createCircle(
                userKey,
                tokenAsset,
                Number(amountInBaseUnits), // Back to number for the TS interface (assuming BigInt support is simplified)
                membersArray,
                cycleIntervalSecs,
                joinDeadlineSecs
            );

            setTxStatus(result.status);

        } catch (err: any) {
            console.error("Creation failed:", err);
            setError(`Transaction failed: ${err.message || String(err)}`);
        } finally {
            setLoading(false);
        }
    };

    return (
        <div className="container mx-auto p-4 max-w-2xl">
            <h1 className="text-3xl font-bold mb-6 text-gray-800">🚀 Create Savings Circle</h1>
            <p className="mb-6 text-gray-600">
                Define the parameters for your Tokenized ROSCA. The creator is the **Circle Owner** and will have admin rights.
            </p>

            <form onSubmit={handleSubmit} className="space-y-6 bg-white p-8 rounded-xl shadow-lg">
                
                {/* Token Asset ID */}
                <div>
                    <label htmlFor="tokenAsset" className="block text-sm font-medium text-gray-700">Token Asset ID (e.g., Soroban Token)</label>
                    <input
                        id="tokenAsset"
                        type="text"
                        value={tokenAsset}
                        onChange={(e) => setTokenAsset(e.target.value)}
                        required
                        className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-3 focus:ring-blue-500 focus:border-blue-500"
                        placeholder="G... or C..."
                    />
                </div>

                {/* Deposit Amount */}
                <div>
                    <label htmlFor="depositAmount" className="block text-sm font-medium text-gray-700">Fixed Deposit Amount (per cycle, unit amount)</label>
                    <input
                        id="depositAmount"
                        type="number"
                        value={depositAmount}
                        onChange={(e) => setDepositAmount(Number(e.target.value))}
                        required
                        min="1"
                        className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-3 focus:ring-blue-500 focus:border-blue-500"
                    />
                </div>

                {/* Cycle Interval */}
                <div>
                    <label htmlFor="cycleInterval" className="block text-sm font-medium text-gray-700">Cycle Interval (Hours)</label>
                    <input
                        id="cycleInterval"
                        type="number"
                        value={cycleIntervalHours}
                        onChange={(e) => setCycleIntervalHours(Number(e.target.value))}
                        required
                        min="1"
                        className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-3 focus:ring-blue-500 focus:border-blue-500"
                    />
                    <p className="mt-1 text-xs text-gray-500">The cycle can only be executed once every {cycleIntervalHours} hours.</p>
                </div>

                {/* Member Keys */}
                <div>
                    <label htmlFor="memberKeys" className="block text-sm font-medium text-gray-700">Initial Member Addresses (Comma Separated)</label>
                    <textarea
                        id="memberKeys"
                        value={memberKeys}
                        onChange={(e) => setMemberKeys(e.target.value)}
                        required
                        rows={3}
                        className="mt-1 block w-full border border-gray-300 rounded-md shadow-sm p-3 focus:ring-blue-500 focus:border-blue-500"
                        placeholder="GB..., GC..., GD..."
                    />
                    <p className="mt-1 text-xs text-gray-500">These members must call `join_circle` before the deadline to confirm their spot.</p>
                </div>

                <button
                    type="submit"
                    disabled={!isConnected || loading}
                    className={`w-full py-3 px-4 border border-transparent rounded-md shadow-sm text-lg font-medium text-white ${
                        !isConnected || loading ? 'bg-gray-400 cursor-not-allowed' : 'bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500'
                    }`}
                >
                    {loading ? 'Creating...' : `Create Circle (Signer: ${userKey ? userKey.substring(0, 4) + '...' : 'Connect Wallet'})`}
                </button>
            </form>

            {error && <div className="mt-4 p-3 bg-red-100 text-red-700 rounded-md">{error}</div>}
            {txStatus && <div className="mt-4 p-3 bg-green-100 text-green-700 rounded-md">Status: {txStatus}</div>}
        </div>
    );
};

export default CreateCirclePage;