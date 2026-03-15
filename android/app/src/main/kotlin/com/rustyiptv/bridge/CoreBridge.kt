package com.rustyiptv.bridge

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

// Note: UniFFI-generated classes will be imported from the generated package
// For now, we define stub types that will be replaced by generated bindings

data class ProviderInput(
    val name: String,
    val providerType: String,
    val url: String,
    val username: String?,
    val password: String?
)

data class ChannelSummary(
    val id: Long,
    val name: String,
    val groupTitle: String?,
    val logoUrl: String?,
    val hidden: Boolean,
    val isFavorite: Boolean
)

data class ChannelPage(
    val channels: List<ChannelSummary>,
    val total: UInt,
    val page: UInt,
    val pageSize: UInt
)

data class FilterOptions(
    val presetId: Long?,
    val searchQuery: String?,
    val showHidden: Boolean
)

data class EpgEntry(
    val id: Long,
    val channelTvgId: String,
    val title: String,
    val startTs: Long,
    val endTs: Long,
    val description: String?
)

// Placeholder for the native Rust core
// Will be replaced with actual UniFFI-generated RustyCore when building for Android
class CoreBridge(private val dbPath: String) {
    // TODO: Replace with actual RustyCore(dbPath) when UniFFI bindings are generated

    suspend fun addProvider(input: ProviderInput): Long =
        withContext(Dispatchers.IO) {
            // TODO: core.addProvider(input)
            0L
        }

    suspend fun importM3uFromUrl(providerId: Long, url: String) =
        withContext(Dispatchers.IO) {
            // TODO: core.importM3uFromUrl(providerId, url)
        }

    suspend fun getChannels(page: UInt, opts: FilterOptions): ChannelPage =
        withContext(Dispatchers.IO) {
            // TODO: core.getChannels(page, opts)
            ChannelPage(emptyList(), 0u, page, 50u)
        }

    suspend fun searchChannels(query: String, page: UInt): List<ChannelSummary> =
        withContext(Dispatchers.IO) {
            // TODO: core.searchChannels(query, page)
            emptyList()
        }

    suspend fun getStreamUrl(channelId: Long): String =
        withContext(Dispatchers.IO) {
            // TODO: core.getStreamUrl(channelId)
            ""
        }

    suspend fun toggleFavorite(channelId: Long): Boolean =
        withContext(Dispatchers.IO) {
            // TODO: core.toggleFavorite(channelId)
            false
        }

    suspend fun toggleHidden(channelId: Long): Boolean =
        withContext(Dispatchers.IO) {
            // TODO: core.toggleHidden(channelId)
            false
        }

    suspend fun getEpg(tvgId: String, fromTs: Long, toTs: Long): List<EpgEntry> =
        withContext(Dispatchers.IO) {
            // TODO: core.getEpg(tvgId, fromTs, toTs)
            emptyList()
        }

    suspend fun getGroups(providerId: Long): List<String> =
        withContext(Dispatchers.IO) {
            // TODO: core.getGroups(providerId)
            emptyList()
        }
}
