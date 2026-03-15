package com.rustyiptv.bridge

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch
import java.io.File

class CoreViewModel(app: Application) : AndroidViewModel(app) {
    private val dbPath = File(app.filesDir, "rusty_iptv.db").absolutePath
    val bridge = CoreBridge(dbPath)

    private val _channels = MutableStateFlow<ChannelPage?>(null)
    val channels: StateFlow<ChannelPage?> = _channels.asStateFlow()

    private val _searchResults = MutableStateFlow<List<ChannelSummary>>(emptyList())
    val searchResults: StateFlow<List<ChannelSummary>> = _searchResults.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _loading = MutableStateFlow(false)
    val loading: StateFlow<Boolean> = _loading.asStateFlow()

    fun loadChannels(page: UInt = 0u, opts: FilterOptions = FilterOptions(null, null, false)) {
        viewModelScope.launch {
            try {
                _loading.value = true
                _channels.value = bridge.getChannels(page, opts)
            } catch (e: Exception) {
                _error.value = e.message
            } finally {
                _loading.value = false
            }
        }
    }

    fun search(query: String) {
        viewModelScope.launch {
            try {
                _searchResults.value = bridge.searchChannels(query, 0u)
            } catch (e: Exception) {
                _error.value = e.message
            }
        }
    }

    fun toggleFavorite(channelId: Long) {
        viewModelScope.launch {
            try {
                bridge.toggleFavorite(channelId)
                _channels.value?.let { loadChannels(it.page) }
            } catch (e: Exception) {
                _error.value = e.message
            }
        }
    }

    fun toggleHidden(channelId: Long) {
        viewModelScope.launch {
            try {
                bridge.toggleHidden(channelId)
                _channels.value?.let { loadChannels(it.page) }
            } catch (e: Exception) {
                _error.value = e.message
            }
        }
    }
}
