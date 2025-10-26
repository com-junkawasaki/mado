import { Page } from '@playwright/test';

export class SoftKVMTestHelper {
  constructor(private page: Page) {}

  /**
   * Wait for the application to be fully loaded
   */
  async waitForAppReady(timeout = 10000): Promise<void> {
    await this.page.waitForTimeout(2000); // Basic wait for initialization

    // Wait for any loading indicators to disappear
    try {
      await this.page.waitForSelector('[data-testid="loading"]', {
        state: 'hidden',
        timeout: timeout - 2000
      });
    } catch {
      // Loading indicator might not exist, continue
    }
  }

  /**
   * Get application status through Tauri invoke
   */
  async getAppStatus(): Promise<any> {
    return await this.page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('get_app_status');
      } catch (error) {
        console.warn('Failed to get app status:', error);
        return null;
      }
    });
  }

  /**
   * Get plugin status
   */
  async getPluginStatus(pluginName: string): Promise<any> {
    return await this.page.evaluate(async (plugin) => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke(`get_${plugin}_status`);
      } catch (error) {
        console.warn(`Failed to get ${plugin} status:`, error);
        return null;
      }
    }, pluginName);
  }

  /**
   * Start input capture
   */
  async startInputCapture(): Promise<boolean> {
    return await this.page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        const result = await window.__TAURI__.invoke('start_input_capture', {
          keyboardEnabled: true,
          mouseEnabled: true
        });
        return result.success === true;
      } catch (error) {
        console.warn('Failed to start input capture:', error);
        return false;
      }
    });
  }

  /**
   * Stop input capture
   */
  async stopInputCapture(): Promise<boolean> {
    return await this.page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        const result = await window.__TAURI__.invoke('stop_input_capture');
        return result.success === true;
      } catch (error) {
        console.warn('Failed to stop input capture:', error);
        return false;
      }
    });
  }

  /**
   * Start protocol server
   */
  async startProtocolServer(address: string): Promise<boolean> {
    return await this.page.evaluate(async (addr) => {
      try {
        // @ts-ignore - Tauri invoke
        const result = await window.__TAURI__.invoke('start_protocol_server', { address: addr });
        return result.success === true;
      } catch (error) {
        console.warn('Failed to start protocol server:', error);
        return false;
      }
    }, address);
  }

  /**
   * Connect to protocol server
   */
  async connectToServer(address: string): Promise<boolean> {
    return await this.page.evaluate(async (addr) => {
      try {
        // @ts-ignore - Tauri invoke
        const result = await window.__TAURI__.invoke('connect_to_server', { address: addr });
        return result.success === true;
      } catch (error) {
        console.warn('Failed to connect to server:', error);
        return false;
      }
    }, address);
  }

  /**
   * Send test input event
   */
  async sendTestInputEvent(): Promise<boolean> {
    return await this.page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        const result = await window.__TAURI__.invoke('send_keyboard_event', {
          keyCode: 65, // 'A'
          pressed: true,
          modifiers: 0
        });
        return result.success === true;
      } catch (error) {
        console.warn('Failed to send test input event:', error);
        return false;
      }
    });
  }

  /**
   * Get connection status
   */
  async getConnectionStatus(): Promise<any> {
    return await this.page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        return await window.__TAURI__.invoke('get_connection_status');
      } catch (error) {
        console.warn('Failed to get connection status:', error);
        return { connected: false };
      }
    });
  }

  /**
   * Wait for connection to be established
   */
  async waitForConnection(timeout = 5000): Promise<boolean> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeout) {
      const status = await this.getConnectionStatus();
      if (status.connected) {
        return true;
      }
      await this.page.waitForTimeout(500);
    }

    return false;
  }

  /**
   * Simulate network error
   */
  async simulateNetworkError(): Promise<void> {
    // This would require backend cooperation to inject errors
    // For now, just disconnect if connected
    await this.page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        await window.__TAURI__.invoke('disconnect');
      } catch (error) {
        console.warn('Failed to disconnect:', error);
      }
    });
  }

  /**
   * Check for error messages in UI
   */
  async getErrorMessages(): Promise<string[]> {
    const errorElements = await this.page.locator('[data-testid="error-message"], .error, .alert-danger').all();
    const messages: string[] = [];

    for (const element of errorElements) {
      const text = await element.textContent();
      if (text) {
        messages.push(text.trim());
      }
    }

    return messages;
  }

  /**
   * Clear any error states
   */
  async clearErrors(): Promise<void> {
    await this.page.evaluate(async () => {
      try {
        // @ts-ignore - Tauri invoke
        await window.__TAURI__.invoke('clear_errors');
      } catch (error) {
        console.warn('Failed to clear errors:', error);
      }
    });
  }
}

export function createTestHelper(page: Page): SoftKVMTestHelper {
  return new SoftKVMTestHelper(page);
}
