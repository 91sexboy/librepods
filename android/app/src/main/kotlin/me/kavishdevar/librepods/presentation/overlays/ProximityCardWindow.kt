package me.kavishdevar.librepods.presentation.overlays

import android.content.Context
import android.content.res.Configuration
import android.graphics.PixelFormat
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Gravity
import android.view.View
import android.view.WindowManager
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.LifecycleRegistry
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.setViewTreeLifecycleOwner
import androidx.lifecycle.setViewTreeViewModelStoreOwner
import androidx.savedstate.SavedStateRegistry
import androidx.savedstate.SavedStateRegistryController
import androidx.savedstate.SavedStateRegistryOwner
import androidx.savedstate.setViewTreeSavedStateRegistryOwner
import me.kavishdevar.librepods.core.devices.AirPodsModel
import me.kavishdevar.librepods.core.devices.AirPodsSpec
import me.kavishdevar.librepods.core.devices.AirPodsSpecs
import me.kavishdevar.librepods.core.devices.Battery
import me.kavishdevar.librepods.data.app.Design
import me.kavishdevar.librepods.data.app.NightTheme
import me.kavishdevar.librepods.data.app.ProximityCardSettings
import me.kavishdevar.librepods.database.app.AppSettingsEntity
import me.kavishdevar.librepods.presentation.design.LibrePodsTheme
import me.kavishdevar.librepods.services.LibrePodsAccessibilityService
import kotlin.math.roundToInt

enum class ProximityCardState {
    VISIBLE,
    CLOSED
}

@Suppress("unused")
class ProximityCardWindow(
    private var context: Context,
    private var appSettings: AppSettingsEntity
) {
    private var windowManager: WindowManager = context.getSystemService(WindowManager::class.java)
    private var composeView: ComposeView? = null
    private var lifecycleOwner: ProxCardLifecycleOwner? = null

    private var autoCloseHandler: Handler? = null
    private var autoCloseRunnable: Runnable? = null

    private var currentAppSettings by mutableStateOf(appSettings)

    private var name by mutableStateOf("AirPods Pro2")
    private var batteries by mutableStateOf(emptySet<Battery>())
    private var spec by mutableStateOf(AirPodsSpecs.getSpec(AirPodsModel.AIRPODS_PRO_3))
    private var onConnect: () -> Unit = {}
    private var airPodsConnected by mutableStateOf(false)

    var usingAccessibilityService by mutableStateOf(false)
        private set

    init {
        usingAccessibilityService = context is LibrePodsAccessibilityService
    }

    var cardState by mutableStateOf(ProximityCardState.CLOSED)
        private set

    val isVisible: Boolean
        get() = composeView?.parent != null && composeView?.visibility == View.VISIBLE

    fun updateContext(context: Context) {
        if (Looper.myLooper() != Looper.getMainLooper()) {
            Handler(Looper.getMainLooper()).post { updateContext(context) }
            return
        }

        this.context = context
        this.usingAccessibilityService = context is LibrePodsAccessibilityService
        this.windowManager = context.getSystemService(WindowManager::class.java)

        updateWindowParams()

        composeView?.let { view ->
            if (view.parent != null) {
                windowManager.removeView(view)
            }
        }

        setupComposeView()

        if (cardState == ProximityCardState.VISIBLE) {
            if (composeView?.parent == null) {
                val params = createLayoutParams()
                try {
                    lifecycleOwner?.start()
                    windowManager.addView(composeView, params)
                    composeView?.visibility = View.VISIBLE
                } catch (e: Exception) {
                    Log.e("ProximityCardWindow", "Error adding overlay view: $e")
                }
            }
        }
    }

    fun updateSettings(appSettings: AppSettingsEntity) {
        if (Looper.myLooper() != Looper.getMainLooper()) {
            Handler(Looper.getMainLooper()).post { updateSettings(appSettings) }
            return
        }

        this.appSettings = appSettings
        this.currentAppSettings = appSettings

        updateWindowParams()
    }

    fun updateBatteries(batteries: Set<Battery>) {
        this.batteries = batteries
    }

    fun updateConnectionState(airPodsConnected: Boolean) {
        this.airPodsConnected = airPodsConnected
    }

    fun show(
        name: String = this.name,
        batteries: Set<Battery> = this.batteries,
        spec: AirPodsSpec = AirPodsSpecs.getSpec(AirPodsModel.AIRPODS_PRO_3),
        onConnect: () -> Unit = this.onConnect,
        airPodsConnected: Boolean = this.airPodsConnected
    ) {
        if (Looper.myLooper() != Looper.getMainLooper()) {
            Handler(Looper.getMainLooper()).post {
                show(name, batteries, spec, onConnect, airPodsConnected)
            }
            return
        }

        this.name = name
        this.batteries = batteries
        this.spec = spec
        this.onConnect = onConnect
        this.cardState = ProximityCardState.VISIBLE
        this.airPodsConnected = airPodsConnected

        if (composeView == null) {
            setupComposeView()
        }

        if (composeView?.parent == null) {
            val params = createLayoutParams()
            try {
                lifecycleOwner?.start()
                windowManager.addView(composeView, params)
                composeView?.visibility = View.VISIBLE
            } catch (e: Exception) {
                Log.e("ProximityCardWindow", "Error adding overlay view: $e")
            }
        } else {
            updateWindowParams()
        }
    }

    private fun updateWindowParams() {
        if (composeView?.parent != null) {
            val params = createLayoutParams()
            windowManager.updateViewLayout(composeView, params)
        }
    }

    private fun isTablet(): Boolean {
        return (context.resources.configuration.screenLayout and Configuration.SCREENLAYOUT_SIZE_MASK) >= Configuration.SCREENLAYOUT_SIZE_XLARGE
    }

    private fun createLayoutParams(): WindowManager.LayoutParams {
        val windowType = if (context is android.accessibilityservice.AccessibilityService) {
            WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY
        } else {
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
        }

        val cardSettings = appSettings.proximityCardSettings

        val isTabletDevice = isTablet() && cardSettings.centerOnTablet

        return WindowManager.LayoutParams(
            cardSettings.width.toInt(),
            cardSettings.height.toInt(),
            windowType,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                WindowManager.LayoutParams.FLAG_SPLIT_TOUCH or
                WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN or
                WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
            PixelFormat.TRANSPARENT
        ).apply {
            gravity = if (isTabletDevice) {
                Gravity.CENTER
            } else {
                Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL
            }

            x = 0
            y = if (isTabletDevice) 0 else cardSettings.offsetY.roundToInt()

            layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
        }
    }

    private fun setupComposeView() {
        lifecycleOwner = ProxCardLifecycleOwner()

        composeView = ComposeView(context).apply {
            setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnViewTreeLifecycleDestroyed)

            setViewTreeLifecycleOwner(lifecycleOwner)
            setViewTreeSavedStateRegistryOwner(lifecycleOwner)
            setViewTreeViewModelStoreOwner(lifecycleOwner)

            setContent {
                val darkTheme = when (currentAppSettings.nightMode) {
                    NightTheme.Dark -> true
                    NightTheme.Light -> false
                    NightTheme.System -> isSystemInDarkTheme()
                }

                LibrePodsTheme(
                    design = Design(
                        system = currentAppSettings.designSystem,
                        colorStyle = currentAppSettings.colorStyle,
                        overrideMaterialColor = currentAppSettings.overrideMaterialColor,
                        contrastLevel = currentAppSettings.contrastLevel,
                        iconStyle = currentAppSettings.iconStyle
                    ),
                    accessibilitySettings = currentAppSettings.accessibilitySettings,
                    fontSettings = currentAppSettings.fontSettings,
                    darkTheme = darkTheme
                ) {
                    ProximityCardContent(
                        name = name,
                        batteries = batteries,
                        settings = appSettings.proximityCardSettings,

                        primaryImageRes = spec.primaryImageRes,
                        caseImageRes = spec.caseImageRes,
                        videoResId = spec.videoResId,

                        cardState = cardState,
                        onConnect = onConnect,
                        onClosed = {
                            cleanupAndRemoveView()
                        },
                        airPodsConnected = airPodsConnected
                    )
                }
            }
        }
    }

    fun close() {
        if (Looper.myLooper() != Looper.getMainLooper()) {
            Handler(Looper.getMainLooper()).post { close() }
            return
        }

        autoCloseRunnable?.let { autoCloseHandler?.removeCallbacks(it) }
        cardState = ProximityCardState.CLOSED


    }

    private fun cleanupAndRemoveView() {
        try {
            composeView?.let { view ->
                if (view.parent != null) {
                    windowManager.removeView(view)
                }
            }
            lifecycleOwner?.stop()
        } catch (e: Exception) {
            Log.e("ProximityCardWindow", "Error removing view: $e")
        } finally {
            composeView = null
            lifecycleOwner = null
            cardState = ProximityCardState.CLOSED
        }
    }
}

private class ProxCardLifecycleOwner: LifecycleOwner, ViewModelStoreOwner, SavedStateRegistryOwner {
    private val savedStateRegistryController = SavedStateRegistryController.create(this)
    private val store = ViewModelStore()

    override val lifecycle: Lifecycle
        field = LifecycleRegistry(this)

    override val savedStateRegistry: SavedStateRegistry
        get() = savedStateRegistryController.savedStateRegistry

    override val viewModelStore: ViewModelStore
        get() = store

    fun start() {
        savedStateRegistryController.performRestore(null)
        lifecycle.handleLifecycleEvent(Lifecycle.Event.ON_CREATE)
        lifecycle.handleLifecycleEvent(Lifecycle.Event.ON_START)
        lifecycle.handleLifecycleEvent(Lifecycle.Event.ON_RESUME)
    }

    fun stop() {
        lifecycle.handleLifecycleEvent(Lifecycle.Event.ON_PAUSE)
        lifecycle.handleLifecycleEvent(Lifecycle.Event.ON_STOP)
        lifecycle.handleLifecycleEvent(Lifecycle.Event.ON_DESTROY)
        store.clear()
    }
}
