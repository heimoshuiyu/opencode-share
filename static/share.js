class ShareRenderer {
  constructor() {
    this.shareId = window.SHARE_ID;
    this.data = null;
    this.init();
  }

  async init() {
    try {
      await this.loadShareData();
      this.renderShare();
    } catch (error) {
      console.error('Failed to load share:', error);
      this.showError();
    }
  }

  async loadShareData() {
    const response = await fetch(`/api/share/${this.shareId}/data`);
    if (!response.ok) {
      if (response.status === 404) {
        throw new Error('Share not found');
      }
      throw new Error('Failed to load share data');
    }

    const rawData = await response.json();
    this.data = this.processShareData(rawData);
  }

  processShareData(rawData) {
    const result = {
      shareId: this.shareId,
      session: null,
      messages: {},
      parts: {},
      diffs: null,
      models: [],
      sessionStatus: { type: 'idle' }
    };

    for (const item of rawData) {
      switch (item.type) {
        case 'session':
          result.session = item.data;
          break;
        case 'message':
          if (!result.messages[item.data.sessionID]) {
            result.messages[item.data.sessionID] = [];
          }
          result.messages[item.data.sessionID].push(item.data);
          break;
        case 'part':
          if (!result.parts[item.data.messageID]) {
            result.parts[item.data.messageID] = [];
          }
          result.parts[item.data.messageID].push(item.data);
          break;
        case 'session_diff':
          result.diffs = item.data;
          break;
        case 'model':
          result.models.push(item.data);
          break;
      }
    }

    // Sort messages by creation time
    for (const sessionId in result.messages) {
      result.messages[sessionId].sort((a, b) => a.time.created - b.time.created);
    }

    return result;
  }

  // Extract message content from message data
  getMessageContent(message) {
    // Check if message has content directly or in parts
    if (message.content) {
      return message.content;
    }
    
    // Get all parts for this message
    const parts = this.data.parts[message.id] || [];
    if (parts.length === 0) {
      // No parts, provide basic message info
      const role = message.role || 'unknown';
      const model = message.modelID || 'unknown model';
      const finish = message.finish || 'unknown';
      return `[${role} message via ${model} - ${finish}]`;
    }
    
    // Process parts to build content
    let content = '';
    
    // Sort parts by time if available, otherwise by type
    const sortedParts = parts.sort((a, b) => {
      if (a.time && b.time) {
        return (a.time.start || a.time.end || 0) - (b.time.start || b.time.end || 0);
      }
      // Fallback: prioritize certain types
      const typeOrder = {
        'step-start': 0,
        'reasoning': 1,
        'text': 2,
        'tool': 3,
        'tool-call': 3,
        'step-finish': 4
      };
      return (typeOrder[a.type] || 99) - (typeOrder[b.type] || 99);
    });
    
    for (const part of sortedParts) {
      switch (part.type) {
        case 'text':
          if (part.text) {
            content += part.text + '\n\n';
          }
          break;

        case 'reasoning':
          if (part.text && part.text.trim()) {
            const reasoningId = `reasoning-${Math.random().toString(36).substr(2, 9)}`;
            const isLongReasoning = part.text.trim().split('\n').length > 5;

            content += `<div class="reasoning-block" id="${reasoningId}">`;
            content += `<div class="reasoning-header">`;
            content += `<span class="reasoning-icon">🤔</span>`;
            content += `<span class="reasoning-title">Thinking Process</span>`;
            if (isLongReasoning) {
              content += `<button class="reasoning-toggle" onclick="toggleReasoning('${reasoningId}')">`;
              content += `<span class="show-text">Show details</span>`;
              content += `<span class="hide-text" style="display:none">Hide</span>`;
              content += `</button>`;
            }
            content += `</div>`;

            if (isLongReasoning) {
              content += `<div class="reasoning-content collapsible" style="display:none;">`;
              content += `<pre><code>${this.escapeHtml(part.text.trim())}</code></pre>`;
              content += `</div>`;
            } else {
              content += `<div class="reasoning-content">`;
              content += `<pre><code>${this.escapeHtml(part.text.trim())}</code></pre>`;
              content += `</div>`;
            }

            content += `</div>`;
          }
          break;

        case 'tool':
        case 'tool-call':
          if (part.state) {
            const toolInfo = this.formatToolCall(part.state);
            content += toolInfo;
          }
          break;

        case 'step-start':
          content += `<div class="step-marker step-start">🚀 Step Started</div>\n\n`;
          break;

        case 'step-finish':
          if (part.tokens) {
            const tokenInfo = this.formatTokenUsage(part.tokens);
            content += `<div class="step-marker step-complete">✅ Step Completed ${tokenInfo}</div>\n\n`;
          } else {
            content += `<div class="step-marker step-complete">✅ Step Completed</div>\n\n`;
          }
          break;

        default:
          // Handle unknown part types
          if (part.text) {
            content += `📝 **${part.type}:**\n${part.text}\n\n`;
          }
      }
    }
    
    return content.trim() || `[${message.role || 'unknown'} message - no content]`;
  }
  
  // Format tool call information with enhanced styling
  formatToolCall(state) {
    // Try multiple possible fields for tool name
    const title = state.title ||
                  state.name ||
                  state.tool ||
                  state.toolName ||
                  state.metadata?.name ||
                  state.metadata?.toolName ||
                  state.metadata?.title ||
                  state.metadata?.description ||
                  'Unknown Tool';
    const status = state.status || 'unknown';

    // Generate a unique ID for this tool call
    const toolId = `tool-${Math.random().toString(36).substr(2, 9)}`;
    const hasOutput = state.output && state.output.trim();
    const outputLines = hasOutput ? state.output.trim().split('\n').length : 0;
    const isLongOutput = outputLines > 10;

    let result = `<div class="tool-call" data-status="${status}">`;
    result += `<div class="tool-header">`;
    result += `<span class="tool-icon">🔧</span>`;
    result += `<span class="tool-title">${this.escapeHtml(title)}</span>`;
    result += `<span class="tool-status status-${status}">${this.escapeHtml(status)}</span>`;
    result += `</div>`;

    // Tool details section
    result += `<div class="tool-details">`;

    // Add input
    if (state.input) {
      result += `<div class="tool-input">`;
      result += `<div class="tool-label">Input</div>`;
      if (typeof state.input === 'string') {
        result += `<code class="tool-inline-code">${this.escapeHtml(state.input)}</code>`;
      } else if (state.input.command) {
        result += `<div class="tool-command">`;
        result += `<span class="prompt">$</span>`;
        result += `<code>${this.escapeHtml(state.input.command)}</code>`;
        result += `</div>`;
        if (state.input.description) {
          result += `<div class="tool-description">${this.escapeHtml(state.input.description)}</div>`;
        }
      } else {
        result += `<pre class="tool-json"><code>${this.escapeHtml(JSON.stringify(state.input, null, 2))}</code></pre>`;
      }
      result += `</div>`;
    }

    // Add output with collapsible functionality
    if (hasOutput) {
      const outputClass = isLongOutput ? 'tool-output tool-output-collapsible' : 'tool-output';
      const preview = isLongOutput ? state.output.trim().split('\n').slice(0, 5).join('\n') : state.output.trim();
      const fullOutput = state.output.trim();

      result += `<div class="${outputClass}" id="${toolId}">`;
      result += `<div class="tool-label">Output`;

      if (isLongOutput) {
        result += `<button class="tool-expand-btn" onclick="this.closest('.tool-output').classList.toggle('expanded')">`;
        result += `<span class="expand-text">Show full output (${outputLines} lines)</span>`;
        result += `<span class="collapse-text">Show less</span>`;
        result += `</button>`;
      }

      result += `</div>`;

      if (isLongOutput) {
        result += `<div class="output-preview"><pre><code>${this.escapeHtml(preview)}</code></pre></div>`;
        result += `<div class="output-full" style="display:none;"><pre><code>${this.escapeHtml(fullOutput)}</code></pre></div>`;
      } else {
        result += `<pre><code>${this.escapeHtml(fullOutput)}</code></pre>`;
      }

      result += `</div>`;
    }

    // Add metadata (exit code, duration, etc.)
    const metadata = [];
    if (state.metadata && state.metadata.exit !== undefined) {
      const exitCode = state.metadata.exit;
      const exitClass = exitCode === 0 ? 'exit-success' : 'exit-error';
      metadata.push(`<span class="${exitClass}">Exit: ${exitCode}</span>`);
    }

    if (state.time) {
      const start = state.time.start;
      const end = state.time.end;
      if (start && end) {
        const duration = end - start;
        metadata.push(`<span class="tool-duration">⏱ ${duration}ms</span>`);
      }
    }

    if (metadata.length > 0) {
      result += `<div class="tool-metadata">${metadata.join(' • ')}</div>`;
    }

    result += `</div>`; // Close tool-details
    result += `</div>`; // Close tool-call

    return result;
  }
  
  // Format token usage information
  formatTokenUsage(tokens) {
    const parts = [];
    
    if (tokens.input) parts.push(`📥 Input: ${tokens.input}`);
    if (tokens.output) parts.push(`📤 Output: ${tokens.output}`);
    if (tokens.reasoning) parts.push(`🧠 Reasoning: ${tokens.reasoning}`);
    
    if (tokens.cache) {
      if (tokens.cache.read) parts.push(`💾 Cache Read: ${tokens.cache.read}`);
      if (tokens.cache.write) parts.push(`💾 Cache Write: ${tokens.cache.write}`);
    }
    
    if (tokens.cost) {
      parts.push(`💰 Cost: $${tokens.cost.toFixed(6)}`);
    }
    
    return parts.length > 0 ? `(${parts.join(', ')})` : '';
  }

  renderShare() {
    const app = document.getElementById('app');
    app.innerHTML = this.generateShareHTML();
    this.attachEventListeners();
  }

  generateShareHTML() {
    const { session, messages, diffs } = this.data;
    if (!session) {
      return this.generateErrorHTML();
    }

    const sessionId = session.id;
    const sessionMessages = messages[sessionId] || [];
    
    // Show all messages, not just user messages
    const allMessages = sessionMessages;

    return `
      <div class="share-container">
        <header class="header">
          <div class="header-left">
            <h1>Opencode Share</h1>
          </div>
          <div class="header-actions">
            <button onclick="window.open('https://github.com/sst/opencode', '_blank')">
              GitHub
            </button>
            <button onclick="window.open('https://opencode.ai/discord', '_blank')">
              Discord
            </button>
          </div>
        </header>

        <div class="content">
          <div class="session-info">
            <div class="session-title">${this.escapeHtml(session.title)}</div>
            <div class="session-meta">
              <span>v${session.version || '1.0.0'}</span>
              <span>•</span>
              <span>${new Date(session.time.created).toLocaleDateString()}</span>
              ${session.directory ? `<span>•</span><span>${this.escapeHtml(session.directory)}</span>` : ''}
              ${session.summary && session.summary.files ? `<span>•</span><span>${session.summary.files} files</span>` : ''}
            </div>
          </div>

          <div class="session-stats">
            <div class="stat-item">
              <span class="stat-label">Messages:</span>
              <span class="stat-value">${allMessages.length}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">Session:</span>
              <span class="stat-value">${this.escapeHtml(sessionId)}</span>
            </div>
          </div>

          <div class="session-messages">
            ${allMessages.map(message => this.renderMessage(message)).join('')}
          </div>

          ${diffs && diffs.length > 0 ? `
            <div class="diff-container">
              <div class="diff-header">${diffs.length} Files Changed</div>
              ${diffs.map(diff => this.renderDiff(diff)).join('')}
            </div>
          ` : ''}
        </div>
      </div>
    `;
  }

  renderMessage(message) {
    const content = this.getMessageContent(message);
    const role = message.role || 'unknown';
    const model = message.modelID || '';
    const timestamp = message.time ? new Date(message.time.created).toLocaleString() : '';
    const tokens = message.tokens || {};

    // Check if content contains HTML (tool calls)
    const hasHtml = content.includes('<div class="tool-call"');

    return `
      <div class="message">
        <div class="message-header">
          <div class="message-role">${role === 'user' ? '👤 User' : '🤖 Assistant'}</div>
          <div class="message-meta">
            ${model ? `<span class="message-model">${model}</span>` : ''}
            <span class="message-time">${timestamp}</span>
            ${tokens.input || tokens.output ? `
              <span class="message-tokens">
                ${tokens.input ? `📥 ${tokens.input}` : ''}
                ${tokens.output ? `📤 ${tokens.output}` : ''}
              </span>
            ` : ''}
          </div>
        </div>
        <div class="message-content ${hasHtml ? 'has-rich-content' : ''}">${hasHtml ? content : this.escapeHtml(content)}</div>
      </div>
    `;
  }

  renderDiff(diff) {
    const lines = this.formatDiff(diff);
    
    return `
      <div class="diff-file">
        <div class="diff-file-name">${this.escapeHtml(diff.file)}</div>
        <div class="diff-content">
          <pre>${lines.map(line => 
            `<div class="diff-line-${line.type}">${this.escapeHtml(line.content)}</div>`
          ).join('')}</pre>
        </div>
      </div>
    `;
  }

  formatDiff(diff) {
    // Simple diff formatting - in a real implementation, you'd use a proper diff library
    const lines = [];
    
    if (diff.before && diff.after) {
      const beforeLines = diff.before.split('\n');
      const afterLines = diff.after.split('\n');
      
      // This is a very simplified diff view
      const maxLines = Math.max(beforeLines.length, afterLines.length);
      
      for (let i = 0; i < maxLines; i++) {
        const beforeLine = beforeLines[i];
        const afterLine = afterLines[i];
        
        if (beforeLine === afterLine) {
          if (beforeLine !== undefined) {
            lines.push({ type: 'unchanged', content: ` ${beforeLine}` });
          }
        } else if (beforeLine === undefined) {
          lines.push({ type: 'added', content: `+${afterLine}` });
        } else if (afterLine === undefined) {
          lines.push({ type: 'removed', content: `-${beforeLine}` });
        } else {
          lines.push({ type: 'removed', content: `-${beforeLine}` });
          lines.push({ type: 'added', content: `+${afterLine}` });
        }
      }
    }
    
    return lines;
  }

  escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  attachEventListeners() {
    // Add any interactive functionality here
    const messageElements = document.querySelectorAll('.message');
    messageElements.forEach(element => {
      element.addEventListener('click', () => {
        // Handle message selection if needed
      });
    });
  }

  showError() {
    document.getElementById('app').style.display = 'none';
    document.getElementById('error-container').style.display = 'flex';
  }

  generateErrorHTML() {
    return `
      <div class="error-content">
        <h1>Share Data Missing</h1>
        <p>The share data is incomplete or corrupted.</p>
        <a href="/">Go Home</a>
      </div>
    `;
  }
}

// Global function for toggling reasoning blocks
function toggleReasoning(id) {
  const block = document.getElementById(id);
  const button = block.querySelector('.reasoning-toggle');
  const content = block.querySelector('.reasoning-content');
  const showText = button.querySelector('.show-text');
  const hideText = button.querySelector('.hide-text');

  if (content.style.display === 'none') {
    content.style.display = 'block';
    showText.style.display = 'none';
    hideText.style.display = 'inline';
  } else {
    content.style.display = 'none';
    showText.style.display = 'inline';
    hideText.style.display = 'none';
  }
}

// Initialize the share renderer when the page loads
document.addEventListener('DOMContentLoaded', () => {
  new ShareRenderer();
});