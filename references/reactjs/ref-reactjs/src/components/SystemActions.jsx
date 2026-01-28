import { useState } from 'react';
import Modal from './Modal';

function SystemActions({ onAction }) {
  const [showRebootModal, setShowRebootModal] = useState(false);
  const [showShutdownModal, setShowShutdownModal] = useState(false);
  const [isRebooting, setIsRebooting] = useState(false);
  const [isShuttingDown, setIsShuttingDown] = useState(false);

  const handleReboot = async () => {
    setIsRebooting(true);
    try {
      const response = await fetch('/api/system/reboot', { method: 'POST' });
      const data = await response.json();
      if (data.success) {
        onAction?.('success', 'System is rebooting...');
      }
    } catch (err) {
      onAction?.('error', 'Failed to initiate reboot');
    } finally {
      setShowRebootModal(false);
      setIsRebooting(false);
    }
  };

  const handleShutdown = async () => {
    setIsShuttingDown(true);
    try {
      const response = await fetch('/api/system/shutdown', { method: 'POST' });
      const data = await response.json();
      if (data.success) {
        onAction?.('success', 'System is shutting down...');
      }
    } catch (err) {
      onAction?.('error', 'Failed to initiate shutdown');
    } finally {
      setShowShutdownModal(false);
      setIsShuttingDown(false);
    }
  };

  return (
    <>
      <div className="flex items-center gap-2">
        <button
          onClick={() => setShowRebootModal(true)}
          className="flex items-center gap-2 px-4 py-2 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded-lg text-sm text-white transition-all hover:border-amber-500/50"
        >
          <svg className="w-4 h-4 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          Reboot
        </button>
        <button
          onClick={() => setShowShutdownModal(true)}
          className="flex items-center gap-2 px-4 py-2 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded-lg text-sm text-white transition-all hover:border-red-500/50"
        >
          <svg className="w-4 h-4 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
          </svg>
          Shutdown
        </button>
      </div>

      {/* Reboot Modal */}
      <Modal
        isOpen={showRebootModal}
        onClose={() => setShowRebootModal(false)}
        title="Confirm Reboot"
        variant="warning"
      >
        <div className="space-y-4">
          <div className="flex items-start gap-3">
            <div className="p-2 bg-amber-500/10 rounded-lg">
              <svg className="w-6 h-6 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
            </div>
            <div>
              <p className="text-white">Are you sure you want to reboot the system?</p>
              <p className="text-sm text-zinc-400 mt-1">
                All running processes will be terminated and the system will restart.
              </p>
            </div>
          </div>
          <div className="flex justify-end gap-3">
            <button
              onClick={() => setShowRebootModal(false)}
              className="px-4 py-2 text-sm text-zinc-400 hover:text-white transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={handleReboot}
              disabled={isRebooting}
              className="px-4 py-2 bg-amber-600 hover:bg-amber-500 text-white text-sm font-medium rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
            >
              {isRebooting ? (
                <>
                  <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  Rebooting...
                </>
              ) : (
                'Reboot Now'
              )}
            </button>
          </div>
        </div>
      </Modal>

      {/* Shutdown Modal */}
      <Modal
        isOpen={showShutdownModal}
        onClose={() => setShowShutdownModal(false)}
        title="Confirm Shutdown"
        variant="danger"
      >
        <div className="space-y-4">
          <div className="flex items-start gap-3">
            <div className="p-2 bg-red-500/10 rounded-lg">
              <svg className="w-6 h-6 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
              </svg>
            </div>
            <div>
              <p className="text-white">Are you sure you want to shut down the system?</p>
              <p className="text-sm text-zinc-400 mt-1">
                The system will power off completely. You will need physical access to turn it back on.
              </p>
            </div>
          </div>
          <div className="flex justify-end gap-3">
            <button
              onClick={() => setShowShutdownModal(false)}
              className="px-4 py-2 text-sm text-zinc-400 hover:text-white transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={handleShutdown}
              disabled={isShuttingDown}
              className="px-4 py-2 bg-red-600 hover:bg-red-500 text-white text-sm font-medium rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
            >
              {isShuttingDown ? (
                <>
                  <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  Shutting down...
                </>
              ) : (
                'Shutdown Now'
              )}
            </button>
          </div>
        </div>
      </Modal>
    </>
  );
}

export default SystemActions;
